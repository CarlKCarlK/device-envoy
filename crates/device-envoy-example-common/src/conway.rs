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

/// Maximum number of backtracking iterations before giving up on the predecessor search.
const MAX_SEARCH_ITERATIONS: u32 = 500_000;

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

type SearchCommandChannel<const H: usize, const W: usize> = Channel<
    CriticalSectionRawMutex,
    SearchCommand<H, W>,
    SEARCH_COMMAND_CHANNEL_CAPACITY,
>;

type SearchEventChannel<const H: usize, const W: usize> = Channel<
    CriticalSectionRawMutex,
    SearchEvent<H, W>,
    SEARCH_EVENT_CHANNEL_CAPACITY,
>;

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
        conway_ui_loop(&led2d, &ir_kepler, &search_command_channel, &search_event_channel),
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
    board.add_pattern(PATTERNS[pattern_index]);

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
    led2d: &L,
    ir_kepler: &I,
    search_command_channel: &SearchCommandChannel<H, W>,
    search_event_channel: &SearchEventChannel<H, W>,
)
where
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
) -> !
{
    loop {
        let target = match search_command_channel.receive().await {
            SearchCommand::Start(target) => target,
            SearchCommand::Cancel => continue,
        };

        let outcome =
            find_predecessor_worker(&target, search_command_channel, search_event_channel).await;
        search_event_channel.send(SearchEvent::Outcome(outcome)).await;
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
                reset_board_for_pattern(board, *pattern_index, stasis_tracker, empty_tracker);
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
        _ => {}
    }
}

fn reset_board_for_pattern<const H: usize, const W: usize>(
    board: &mut Board<H, W>,
    pattern_index: usize,
    stasis_tracker: &mut (u8, u16),
    empty_tracker: &mut u8,
) {
    let pattern = PATTERNS[pattern_index];
    *board = Board::new();
    board.add_pattern(pattern);
    *stasis_tracker = (0, 0);
    *empty_tracker = 0;
}

fn evaluate_auto_reset<const H: usize, const W: usize>(
    board: &mut Board<H, W>,
    pattern_index: usize,
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
                board.add_pattern(current_pattern);
                *stasis_tracker = (0, 0);
                *empty_tracker = 0;
            }
        } else {
            *stasis_tracker = (1, live_cell_count);
        }
    } else if live_cell_count == 0 {
        *empty_tracker += 1;
        if *empty_tracker >= STASIS_RESET_GENERATIONS {
            board.add_pattern(current_pattern);
            *stasis_tracker = (0, 0);
            *empty_tracker = 0;
        }
    } else {
        *empty_tracker = 0;
    }
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

#[derive(Clone, Copy, Debug)]
enum Pattern {
    Glider,
    Blinker,
    Toad,
    Beacon,
    Lwss,
    Block,
    Pentadecathlon,
    Random,
    Cross,
    Custom9,
}

#[derive(Copy, Clone)]
struct Board<const H: usize, const W: usize> {
    cells: [[bool; W]; H],
}

impl<const H: usize, const W: usize> Board<H, W> {
    fn new() -> Self {
        Self {
            cells: [[false; W]; H],
        }
    }

    fn step(&mut self) {
        let mut next_cells = [[false; W]; H];

        for row_index in 0..H {
            for col_index in 0..W {
                let live_neighbors = self.count_live_neighbors(row_index, col_index);
                let is_alive = self.cells[row_index][col_index];

                next_cells[row_index][col_index] = match (is_alive, live_neighbors) {
                    (true, 2) | (true, 3) => true,
                    (false, 3) => true,
                    _ => false,
                };
            }
        }

        self.cells = next_cells;
    }

    fn count_live_neighbors(&self, row: usize, col: usize) -> u8 {
        let mut live_neighbor_count = 0u8;

        for row_offset in [-1, 0, 1].iter().copied() {
            for col_offset in [-1, 0, 1].iter().copied() {
                if row_offset == 0 && col_offset == 0 {
                    continue;
                }

                let neighbor_row = ((row as isize + row_offset).rem_euclid(H as isize)) as usize;
                let neighbor_col = ((col as isize + col_offset).rem_euclid(W as isize)) as usize;

                if self.cells[neighbor_row][neighbor_col] {
                    live_neighbor_count += 1;
                }
            }
        }

        live_neighbor_count
    }

    fn to_frame(&self, alive_color: RGB8) -> Frame2d<W, H> {
        let mut frame2d = Frame2d::<W, H>::new();
        for row_index in 0..H {
            for col_index in 0..W {
                if self.cells[row_index][col_index] {
                    frame2d[(col_index, row_index)] = alive_color;
                }
            }
        }
        frame2d
    }

    fn add_pattern(&mut self, pattern: Pattern) {
        match pattern {
            Pattern::Glider => self.add_glider(4, 2),
            Pattern::Blinker => self.add_blinker(5, 4),
            Pattern::Toad => self.add_toad(5, 4),
            Pattern::Beacon => self.add_beacon(4, 4),
            Pattern::Lwss => self.add_lwss(5, 6),
            Pattern::Block => self.add_block(5, 4),
            Pattern::Pentadecathlon => self.add_pentadecathlon(),
            Pattern::Random => self.add_random(),
            Pattern::Cross => self.add_cross(7, 7),
            Pattern::Custom9 => self.add_custom9(),
        }
    }

    fn set_alive(&mut self, row_index: usize, col_index: usize) {
        self.cells[row_index % H][col_index % W] = true;
    }

    fn add_glider(&mut self, start_row: usize, start_col: usize) {
        self.set_alive(start_row, start_col + 1);
        self.set_alive(start_row + 1, start_col + 2);
        self.set_alive(start_row + 2, start_col);
        self.set_alive(start_row + 2, start_col + 1);
        self.set_alive(start_row + 2, start_col + 2);
    }

    fn add_blinker(&mut self, row: usize, col: usize) {
        self.set_alive(row, col);
        self.set_alive(row, col + 1);
        self.set_alive(row, col + 2);
    }

    fn add_toad(&mut self, row: usize, col: usize) {
        self.set_alive(row, col + 1);
        self.set_alive(row, col + 2);
        self.set_alive(row, col + 3);
        self.set_alive(row + 1, col);
        self.set_alive(row + 1, col + 1);
        self.set_alive(row + 1, col + 2);
    }

    fn add_beacon(&mut self, row: usize, col: usize) {
        self.set_alive(row, col);
        self.set_alive(row, col + 1);
        self.set_alive(row + 1, col);
        self.set_alive(row + 1, col + 1);
        self.set_alive(row + 2, col + 2);
        self.set_alive(row + 2, col + 3);
        self.set_alive(row + 3, col + 2);
        self.set_alive(row + 3, col + 3);
    }

    fn add_lwss(&mut self, row: usize, col: usize) {
        self.set_alive(row, col + 1);
        self.set_alive(row + 1, col);
        self.set_alive(row + 2, col);
        self.set_alive(row + 2, col + 1);
        self.set_alive(row + 2, col + 2);
        self.set_alive(row + 2, col + 3);
        self.set_alive(row + 1, col + 3);
    }

    fn add_block(&mut self, row: usize, col: usize) {
        self.set_alive(row, col);
        self.set_alive(row, col + 1);
        self.set_alive(row + 1, col);
        self.set_alive(row + 1, col + 1);
    }

    fn add_wall(&mut self, row: usize) {
        for col_index in 0..W {
            self.set_alive(row, col_index);
        }
    }

    fn add_vertical(&mut self, col: usize) {
        for row_index in 0..H {
            self.set_alive(row_index, col);
        }
    }

    fn add_cross(&mut self, row: usize, col: usize) {
        self.add_wall(row);
        self.add_vertical(col);
    }

    fn add_random(&mut self) {
        let now_millis = Instant::now().as_millis();
        let mut random_seed = (now_millis ^ 0x9e37_79b9) as u32;
        for row_index in 0..H {
            for col_index in 0..W {
                random_seed = random_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.cells[row_index][col_index] = (random_seed & 0x100) != 0;
            }
        }
    }

    fn count_live_cells(&self) -> u16 {
        let mut live_cell_count = 0u16;
        for row in &self.cells {
            for &is_alive in row {
                if is_alive {
                    live_cell_count += 1;
                }
            }
        }
        live_cell_count
    }

    fn add_pentadecathlon(&mut self) {
        self.draw_ascii_pattern(
            &[
                "................",
                "................",
                "................",
                "......###.......",
                ".....#...#......",
                "................",
                "....#.....#.....",
                "....#.....#.....",
                "................",
                ".....#...#......",
                "......###.......",
                "................",
                "................",
                "................",
                "................",
                "................",
            ],
            0,
            0,
        );
    }

    fn add_custom9(&mut self) {
        self.draw_ascii_pattern(
            &[
                "................",
                "...##.....##....",
                "....##...##.....",
                ".#..#.#.#.#..#..",
                ".###.##.##.###..",
                "..#.#.#.#.#.#...",
                "...###...###....",
                "................",
                "...###...###....",
                "..#.#.#.#.#.#...",
                ".###.##.##.###..",
                ".#..#.#.#.#..#..",
                "....##...##.....",
                "...##.....##....",
                "................",
                "................",
            ],
            0,
            0,
        );
    }

    fn draw_ascii_pattern(&mut self, rows: &[&str], row_offset: usize, col_offset: usize) {
        for (row_index, row) in rows.iter().enumerate() {
            for (col_index, row_char) in row.chars().enumerate() {
                if row_char == '#' {
                    self.set_alive(row_index + row_offset, col_index + col_offset);
                }
            }
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

/// Check whether any newly-fully-constrained cells in the Moore neighborhood of
/// `(changed_row, changed_col)` are inconsistent with `target`.
///
/// After assigning a cell, up to 9 neighboring cells may have their full 3×3
/// neighborhood complete for the first time.  For each of those cells, verify
/// that applying the Conway rule to `candidate` gives the value in `target`.
/// Returns `true` if all constraints that can be checked are satisfied.
fn check_search_constraints<const W: usize, const H: usize>(
    candidate: &Board<H, W>,
    assigned: &[[bool; W]; H],
    target: &Board<H, W>,
    changed_row: usize,
    changed_col: usize,
) -> bool {
    for dr in [-1i32, 0, 1] {
        for dc in [-1i32, 0, 1] {
            let check_row = ((changed_row as i32 + dr).rem_euclid(H as i32)) as usize;
            let check_col = ((changed_col as i32 + dc).rem_euclid(W as i32)) as usize;

            // Is every cell in the 3x3 neighborhood of (check_row, check_col) assigned?
            let mut neighborhood_complete = true;
            'check_neighborhood: for nr_offset in [-1i32, 0, 1] {
                for nc_offset in [-1i32, 0, 1] {
                    let nr = ((check_row as i32 + nr_offset).rem_euclid(H as i32)) as usize;
                    let nc = ((check_col as i32 + nc_offset).rem_euclid(W as i32)) as usize;
                    if !assigned[nr][nc] {
                        neighborhood_complete = false;
                        break 'check_neighborhood;
                    }
                }
            }

            if neighborhood_complete {
                // Apply the Conway rule to the candidate predecessor at this cell.
                let live_neighbors = candidate.count_live_neighbors(check_row, check_col);
                let is_alive_in_prev = candidate.cells[check_row][check_col];
                let next_alive = matches!(
                    (is_alive_in_prev, live_neighbors),
                    (true, 2) | (true, 3) | (false, 3)
                );
                if next_alive != target.cells[check_row][check_col] {
                    return false;
                }
            }
        }
    }
    true
}

/// Outcome of a predecessor search.
#[derive(Clone, Copy)]
enum SearchOutcome<const H: usize, const W: usize> {
    /// A valid predecessor board was found.
    Found(Board<H, W>),
    /// No predecessor exists (Garden of Eden) or the iteration cap was reached.
    NotFound,
    /// The search was cancelled.
    Cancelled,
}

/// Try to find a predecessor board: a state that evolves into `target` after one step.
///
/// Uses iterative backtracking, assigning predecessor cells one by one in raster order
/// and pruning branches as soon as a Conway-rule constraint is violated.
async fn find_predecessor_worker<const W: usize, const H: usize>(
    target: &Board<H, W>,
    search_command_channel: &SearchCommandChannel<H, W>,
    search_event_channel: &SearchEventChannel<H, W>,
) -> SearchOutcome<H, W>
{
    let mut candidate = Board::<H, W>::new();
    // choices[r][c]: 0 = try false next, 1 = try true next, 2 = both tried (backtrack).
    let mut choices = [[0u8; W]; H];
    let mut assigned = [[false; W]; H];
    let mut depth = 0usize;
    let total = H * W;
    let mut iteration = 0u32;

    loop {
        if depth == total {
            return SearchOutcome::Found(candidate);
        }

        let row = depth / W;
        let col = depth % W;

        let try_value = match choices[row][col] {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };

        if let Some(value) = try_value {
            choices[row][col] += 1;
            candidate.cells[row][col] = value;
            assigned[row][col] = true;

            if check_search_constraints(&candidate, &assigned, target, row, col) {
                depth += 1;
            } else {
                assigned[row][col] = false;
                // choices[row][col] was already incremented; next iteration tries the other value.
            }
        } else {
            // Both values exhausted — backtrack.
            choices[row][col] = 0;
            assigned[row][col] = false;
            if depth == 0 {
                return SearchOutcome::NotFound;
            }
            depth -= 1;
            let prev_row = depth / W;
            let prev_col = depth % W;
            assigned[prev_row][prev_col] = false;
        }

        iteration += 1;
        if iteration >= MAX_SEARCH_ITERATIONS {
            return SearchOutcome::NotFound;
        }

        if let Ok(search_command) = search_command_channel.try_receive() {
            match search_command {
                SearchCommand::Cancel => return SearchOutcome::Cancelled,
                SearchCommand::Start(_) => return SearchOutcome::Cancelled,
            }
        }

        // Periodically publish progress and yield so the controller stays responsive.
        if iteration % SEARCH_CHECK_INTERVAL_ITERATIONS == 0 {
            let _ = search_event_channel.try_send(SearchEvent::Progress {
                candidate,
                assigned,
                target: *target,
            });
            yield_now().await;
        }
    }
}
