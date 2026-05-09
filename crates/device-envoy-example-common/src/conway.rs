use device_envoy_conway_core::{
    Board, Pattern, PredecessorSearch, RandomSymmetryMode, SearchOutcome, SearchStep,
};
use device_envoy_core::{
    ir::kepler::{IrKepler, KeplerKeys},
    led_strip::RGB8,
    led2d::{Frame2d, Led2d},
};
use embassy_futures::{
    join::join,
    select::{Either, select},
    yield_now,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, TrySendError},
};
use embassy_time::{Duration, Instant, Timer};
use smart_leds::colors;

const STASIS_RESET_GENERATIONS: u8 = 15;

/// Color used to visualize the predecessor search in progress.
const SEARCH_COLOR: RGB8 = colors::RED;
/// Color used to show cells that were assigned as dead during search.
const SEARCH_ASSIGNED_DEAD_COLOR: RGB8 = RGB8 { r: 0, g: 0, b: 12 };
/// Color used to hint where the current target generation is alive.
const SEARCH_TARGET_HINT_COLOR: RGB8 = RGB8 { r: 0, g: 10, b: 0 };
/// How many backtracking iterations to run between search UI/input checkpoints.
const SEARCH_CHECK_INTERVAL_ITERATIONS: u32 = 32;
/// Capacity of the command channel from UI controller to search worker.
const SEARCH_COMMAND_CHANNEL_CAPACITY: usize = 2;
/// Capacity of the event channel from search worker to UI controller.
const SEARCH_EVENT_CHANNEL_CAPACITY: usize = 1;

const ALIVE_COLORS: [RGB8; 6] = [
    colors::LIME,
    colors::CYAN,
    colors::MAGENTA,
    colors::ORANGE,
    colors::YELLOW,
    colors::WHITE,
];

const PATTERNS: [Pattern; 10] = [
    Pattern::Glider,
    Pattern::Random,
    Pattern::Blinker,
    Pattern::Toad,
    Pattern::Beacon,
    Pattern::Lwss,
    Pattern::Block,
    Pattern::Pentadecathlon,
    Pattern::Cross,
    Pattern::Custom9,
];

type SearchCommandChannel<const H: usize, const W: usize> =
    Channel<CriticalSectionRawMutex, SearchCommand<H, W>, SEARCH_COMMAND_CHANNEL_CAPACITY>;

type SearchEventChannel<const H: usize, const W: usize> =
    Channel<CriticalSectionRawMutex, SearchEvent<H, W>, SEARCH_EVENT_CHANNEL_CAPACITY>;

#[derive(Clone, Copy)]
enum SearchCommand<const H: usize, const W: usize> {
    Start(Board<H, W>),
    Cancel,
}

#[derive(Clone, Copy)]
enum SearchEvent<const H: usize, const W: usize> {
    Progress {
        candidate: Board<H, W>,
        assigned: [[bool; W]; H],
        target: Board<H, W>,
    },
    Outcome(SearchOutcome<H, W>),
}

pub async fn conway_with_led2d_ir_kepler<const W: usize, const H: usize, L, I>(
    led2d: L,
    ir_kepler: I,
) -> !
where
    L: Led2d<W, H>,
    I: IrKepler,
{
    assert!(W > 0, "Conway width must be greater than zero");
    assert!(H > 0, "Conway height must be greater than zero");

    let search_command_channel = SearchCommandChannel::<H, W>::new();
    let search_event_channel = SearchEventChannel::<H, W>::new();

    let _ = join(
        conway_ui_loop(
            &led2d,
            &ir_kepler,
            &search_command_channel,
            &search_event_channel,
        ),
        conway_search_worker_loop(&search_command_channel, &search_event_channel),
    )
    .await;
    unreachable!()
}

async fn conway_ui_loop<const W: usize, const H: usize, L, I>(
    led2d: &L,
    ir_kepler: &I,
    search_command_channel: &SearchCommandChannel<H, W>,
    search_event_channel: &SearchEventChannel<H, W>,
) -> !
where
    L: Led2d<W, H>,
    I: IrKepler,
{
    let mut board = Board::<H, W>::new();
    let mut pattern_index = 1usize;
    let mut speed_mode = SpeedMode::Medium;
    let mut paused = false;
    let mut display_power = DisplayPower::On;
    let mut color_index = 1usize;
    let mut alive_color = ALIVE_COLORS[color_index];
    let mut random_symmetry_mode = RandomSymmetryMode::None;
    add_pattern(&mut board, PATTERNS[pattern_index], random_symmetry_mode);

    let mut stasis_tracker = (0u8, 0u16);
    let mut empty_tracker = 0u8;

    loop {
        match display_power {
            DisplayPower::On => {
                let frame2d = board.to_frame(alive_color);
                led2d.write_frame(frame2d);

                let frame_duration = speed_mode.frame_duration();

                match select(Timer::after(frame_duration), ir_kepler.wait_for_press()).await {
                    Either::First(_) => {
                        if paused {
                            continue;
                        }

                        board.step();
                        evaluate_auto_reset(
                            &mut board,
                            pattern_index,
                            random_symmetry_mode,
                            &mut stasis_tracker,
                            &mut empty_tracker,
                        );
                    }
                    Either::Second(KeplerKeys::Prev) => {
                        let original_board = board;
                        search_command_channel
                            .send(SearchCommand::Start(board))
                            .await;
                        run_search_session(
                            &mut board,
                            original_board,
                            &mut pattern_index,
                            &mut stasis_tracker,
                            &mut empty_tracker,
                            &mut speed_mode,
                            &mut paused,
                            &mut display_power,
                            &mut color_index,
                            &mut alive_color,
                            &mut random_symmetry_mode,
                            led2d,
                            ir_kepler,
                            search_command_channel,
                            search_event_channel,
                        )
                        .await;
                    }
                    Either::Second(button) => {
                        handle_sync_button(
                            button,
                            &mut board,
                            &mut pattern_index,
                            &mut stasis_tracker,
                            &mut empty_tracker,
                            &mut speed_mode,
                            &mut paused,
                            &mut display_power,
                            &mut color_index,
                            &mut alive_color,
                            &mut random_symmetry_mode,
                            led2d,
                        );
                    }
                }
            }
            DisplayPower::Off => {
                let button = ir_kepler.wait_for_press().await;
                if button == KeplerKeys::Power {
                    display_power = DisplayPower::On;
                    led2d.write_frame(board.to_frame(alive_color));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_search_session<const W: usize, const H: usize, L, I>(
    board: &mut Board<H, W>,
    original_board: Board<H, W>,
    pattern_index: &mut usize,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
    speed_mode: &mut SpeedMode,
    paused: &mut bool,
    display_power: &mut DisplayPower,
    color_index: &mut usize,
    alive_color: &mut RGB8,
    random_symmetry_mode: &mut RandomSymmetryMode,
    led2d: &L,
    ir_kepler: &I,
    search_command_channel: &SearchCommandChannel<H, W>,
    search_event_channel: &SearchEventChannel<H, W>,
) where
    L: Led2d<W, H>,
    I: IrKepler,
{
    let mut cancellation_key = None;
    loop {
        match select(search_event_channel.receive(), ir_kepler.wait_for_press()).await {
            Either::First(SearchEvent::Progress {
                candidate,
                assigned,
                target,
            }) => {
                led2d.write_frame(search_frame(&candidate, &assigned, &target, SEARCH_COLOR));
            }
            Either::First(SearchEvent::Outcome(SearchOutcome::Found(predecessor))) => {
                if cancellation_key.is_none() {
                    *board = predecessor;
                    *stasis_tracker = (0, 0);
                    *empty_tracker = 0;
                } else {
                    *board = original_board;
                    led2d.write_frame(board.to_frame(*alive_color));
                    apply_cancel_button(
                        cancellation_key,
                        board,
                        pattern_index,
                        stasis_tracker,
                        empty_tracker,
                        speed_mode,
                        paused,
                        display_power,
                        color_index,
                        alive_color,
                        random_symmetry_mode,
                        led2d,
                    );
                }
                return;
            }
            Either::First(SearchEvent::Outcome(SearchOutcome::NotFound)) => {
                if cancellation_key.is_some() {
                    *board = original_board;
                    led2d.write_frame(board.to_frame(*alive_color));
                    apply_cancel_button(
                        cancellation_key,
                        board,
                        pattern_index,
                        stasis_tracker,
                        empty_tracker,
                        speed_mode,
                        paused,
                        display_power,
                        color_index,
                        alive_color,
                        random_symmetry_mode,
                        led2d,
                    );
                }
                return;
            }
            Either::First(SearchEvent::Outcome(SearchOutcome::Cancelled)) => {
                *board = original_board;
                led2d.write_frame(board.to_frame(*alive_color));
                apply_cancel_button(
                    cancellation_key,
                    board,
                    pattern_index,
                    stasis_tracker,
                    empty_tracker,
                    speed_mode,
                    paused,
                    display_power,
                    color_index,
                    alive_color,
                    random_symmetry_mode,
                    led2d,
                );
                return;
            }
            Either::Second(key) => {
                if cancellation_key.is_none() {
                    cancellation_key = Some(key);
                    request_search_cancel(search_command_channel);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_cancel_button<const W: usize, const H: usize, L: Led2d<W, H>>(
    cancellation_key: Option<KeplerKeys>,
    board: &mut Board<H, W>,
    pattern_index: &mut usize,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
    speed_mode: &mut SpeedMode,
    paused: &mut bool,
    display_power: &mut DisplayPower,
    color_index: &mut usize,
    alive_color: &mut RGB8,
    random_symmetry_mode: &mut RandomSymmetryMode,
    led2d: &L,
) {
    if let Some(key) = cancellation_key {
        if !matches!(key, KeplerKeys::Prev) {
            handle_sync_button(
                key,
                board,
                pattern_index,
                stasis_tracker,
                empty_tracker,
                speed_mode,
                paused,
                display_power,
                color_index,
                alive_color,
                random_symmetry_mode,
                led2d,
            );
        }
    }
}

fn request_search_cancel<const H: usize, const W: usize>(
    search_command_channel: &SearchCommandChannel<H, W>,
) {
    if let Err(TrySendError::Full(SearchCommand::Cancel)) =
        search_command_channel.try_send(SearchCommand::Cancel)
    {
        // A cancellation command is already queued.
    }
}

async fn conway_search_worker_loop<const W: usize, const H: usize>(
    search_command_channel: &SearchCommandChannel<H, W>,
    search_event_channel: &SearchEventChannel<H, W>,
) -> ! {
    loop {
        let target = match search_command_channel.receive().await {
            SearchCommand::Start(target) => target,
            SearchCommand::Cancel => continue,
        };

        let mut predecessor_search = PredecessorSearch::new(target);
        loop {
            if let Ok(search_command) = search_command_channel.try_receive() {
                match search_command {
                    SearchCommand::Cancel | SearchCommand::Start(_) => {
                        let SearchStep::Outcome(outcome) = predecessor_search.cancel() else {
                            unreachable!();
                        };
                        search_event_channel
                            .send(SearchEvent::Outcome(outcome))
                            .await;
                        break;
                    }
                }
            }

            match predecessor_search.advance(SEARCH_CHECK_INTERVAL_ITERATIONS) {
                SearchStep::Progress {
                    candidate,
                    assigned,
                    target,
                } => {
                    let _ = search_event_channel.try_send(SearchEvent::Progress {
                        candidate,
                        assigned,
                        target,
                    });
                    yield_now().await;
                }
                SearchStep::Outcome(outcome) => {
                    search_event_channel
                        .send(SearchEvent::Outcome(outcome))
                        .await;
                    break;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_sync_button<const W: usize, const H: usize, L: Led2d<W, H>>(
    key: KeplerKeys,
    board: &mut Board<H, W>,
    pattern_index: &mut usize,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
    speed_mode: &mut SpeedMode,
    paused: &mut bool,
    display_power: &mut DisplayPower,
    color_index: &mut usize,
    alive_color: &mut RGB8,
    random_symmetry_mode: &mut RandomSymmetryMode,
    led2d: &L,
) {
    match key {
        KeplerKeys::Power => {
            *display_power = DisplayPower::Off;
            led2d.write_frame(Frame2d::<W, H>::new());
        }
        KeplerKeys::Num(number) => {
            if number < PATTERNS.len() as u8 {
                *pattern_index = number as usize;
                if PATTERNS[*pattern_index] == Pattern::Random {
                    *random_symmetry_mode = RandomSymmetryMode::None;
                }
                reset_board_for_pattern(
                    board,
                    *pattern_index,
                    *random_symmetry_mode,
                    stasis_tracker,
                    empty_tracker,
                );
            }
        }
        KeplerKeys::Minus => {
            *speed_mode = speed_mode.slower();
        }
        KeplerKeys::Plus => {
            *speed_mode = speed_mode.faster();
        }
        KeplerKeys::Next => {
            if *paused {
                board.step();
            }
        }
        KeplerKeys::PlayPause => {
            *paused = !*paused;
        }
        KeplerKeys::Mode => {
            *color_index = (*color_index + 1) % ALIVE_COLORS.len();
            *alive_color = ALIVE_COLORS[*color_index];
        }
        KeplerKeys::Repeat => {
            if PATTERNS[*pattern_index] == Pattern::Random {
                *random_symmetry_mode = random_symmetry_mode.next();
                reset_board_for_pattern(
                    board,
                    *pattern_index,
                    *random_symmetry_mode,
                    stasis_tracker,
                    empty_tracker,
                );
            }
        }
        _ => {}
    }
}

fn reset_board_for_pattern<const H: usize, const W: usize>(
    board: &mut Board<H, W>,
    pattern_index: usize,
    random_symmetry_mode: RandomSymmetryMode,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
) {
    let pattern = PATTERNS[pattern_index];
    *board = Board::new();
    add_pattern(board, pattern, random_symmetry_mode);
    *stasis_tracker = (0, 0);
    *empty_tracker = 0;
}

fn evaluate_auto_reset<const H: usize, const W: usize>(
    board: &mut Board<H, W>,
    pattern_index: usize,
    random_symmetry_mode: RandomSymmetryMode,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
) {
    let live_cell_count = board.count_live_cells();
    let current_pattern = PATTERNS[pattern_index];

    if matches!(current_pattern, Pattern::Random | Pattern::Cross) {
        let (unchanged_count, last_live_count) = *stasis_tracker;
        if live_cell_count == last_live_count {
            let new_unchanged_count = unchanged_count + 1;
            *stasis_tracker = (new_unchanged_count, live_cell_count);

            if new_unchanged_count >= STASIS_RESET_GENERATIONS {
                add_pattern(board, current_pattern, random_symmetry_mode);
                *stasis_tracker = (0, 0);
                *empty_tracker = 0;
            }
        } else {
            *stasis_tracker = (1, live_cell_count);
        }
    } else if live_cell_count == 0 {
        *empty_tracker += 1;
        if *empty_tracker >= STASIS_RESET_GENERATIONS {
            add_pattern(board, current_pattern, random_symmetry_mode);
            *stasis_tracker = (0, 0);
            *empty_tracker = 0;
        }
    } else {
        *empty_tracker = 0;
    }
}

fn add_pattern<const H: usize, const W: usize>(
    board: &mut Board<H, W>,
    pattern: Pattern,
    random_symmetry_mode: RandomSymmetryMode,
) {
    let random_seed = (Instant::now().as_millis() ^ 0x9e37_79b9) as u32;
    board.add_pattern_with_seed_and_random_symmetry(pattern, random_seed, random_symmetry_mode);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeedMode {
    Slow,
    Medium,
    Fast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayPower {
    On,
    Off,
}

impl SpeedMode {
    const fn slower(self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Medium => Self::Slow,
            Self::Fast => Self::Medium,
        }
    }

    const fn faster(self) -> Self {
        match self {
            Self::Slow => Self::Medium,
            Self::Medium => Self::Fast,
            Self::Fast => Self::Fast,
        }
    }

    const fn frame_duration(self) -> Duration {
        match self {
            Self::Slow => Duration::from_millis(500),
            Self::Medium => Duration::from_millis(160),
            Self::Fast => Duration::from_millis(50),
        }
    }
}

/// Build a display frame showing only the assigned cells of a candidate predecessor board.
/// Assigned alive cells appear in `search_color`; assigned dead cells use a dim blue;
/// alive cells in the target generation use a dim green hint.
fn search_frame<const W: usize, const H: usize>(
    candidate: &Board<H, W>,
    assigned: &[[bool; W]; H],
    target: &Board<H, W>,
    search_color: RGB8,
) -> Frame2d<W, H> {
    let mut frame = Frame2d::<W, H>::new();
    for row_index in 0..H {
        for col_index in 0..W {
            if target.cells[row_index][col_index] {
                frame[(col_index, row_index)] = SEARCH_TARGET_HINT_COLOR;
            }

            if assigned[row_index][col_index] {
                frame[(col_index, row_index)] = if candidate.cells[row_index][col_index] {
                    search_color
                } else {
                    SEARCH_ASSIGNED_DEAD_COLOR
                };
            }
        }
    }
    frame
}
