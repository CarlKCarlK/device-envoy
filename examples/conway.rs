#![allow(missing_docs)]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy::ir::{IrKepler, IrKeplerStatic, KeplerButton};
use device_envoy::led_strip::Current;
use device_envoy::led_strip::RGB8;
use device_envoy::led2d;
use device_envoy::led2d::Frame2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use device_envoy::{Error, Result};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::init;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use panic_probe as _;
use smart_leds::colors;

// One 16x16 panel in serpentine column-major order.
const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_column_major();

led2d! {
    Led16x16 {
        pin: PIN_6,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(500),
        max_frames: 30,
        font: Led2dFont::Font4x6Trim,
    }
}

/// Message type for communicating pattern changes and speed adjustments to the Conway task.
#[derive(Clone, Copy, Debug, defmt::Format)]
enum ConwayMessage {
    NextPattern,
    PrevPattern,
    SetSpeed(SpeedMode),
    SetPatternIndex(usize),
    TogglePause,
    NextColor,
}

/// Speed modes for the simulation.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
enum SpeedMode {
    Slower, // 10x slower (500ms per generation)
    Medium, // Log midpoint (~160ms per generation)
    Normal, // 1x normal (50ms per generation)
}

impl SpeedMode {
    const fn slower(self) -> Self {
        match self {
            Self::Slower => Self::Normal,
            Self::Medium => Self::Slower,
            Self::Normal => Self::Medium,
        }
    }

    const fn faster(self) -> Self {
        match self {
            Self::Slower => Self::Medium,
            Self::Medium => Self::Normal,
            Self::Normal => Self::Slower,
        }
    }
}

#[derive(Clone, Copy, Debug, defmt::Format)]
enum Pattern {
    Glider,
    Blinker,
    Toad,
    Beacon,
    LWSS,
    Block,
    Wall,
    Random,
    Cross,
}

const PATTERNS: &[Pattern] = &[
    Pattern::Glider,
    Pattern::Random,
    Pattern::Blinker,
    Pattern::Toad,
    Pattern::Beacon,
    Pattern::LWSS,
    Pattern::Block,
    Pattern::Wall,
    Pattern::Cross,
];

const ALIVE_COLORS: &[RGB8] = &[
    colors::LIME,
    colors::CYAN,
    colors::MAGENTA,
    colors::ORANGE,
    colors::YELLOW,
    colors::WHITE,
];

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Conway's Game of Life on 16x16 LED panel (IR remote on GPIO15)");
    let p = init(Default::default());

    let led16x16 = Led16x16::new(p.PIN_6, p.PIO0, p.DMA_CH0, spawner)?;
    static IR_KEPLER_STATIC: IrKeplerStatic = IrKepler::new_static();
    let ir_kepler = IrKepler::new(&IR_KEPLER_STATIC, p.PIN_15, p.PIO1, spawner)?;

    // Create Conway device with static resources and spawn background task
    static CONWAY_STATIC: ConwayStatic = Conway::new_static();
    let conway = Conway::new(&CONWAY_STATIC, led16x16, spawner)?;

    // Speed mode cycling state
    let mut speed_mode = SpeedMode::Slower;

    // Main loop: handle IR remote input
    loop {
        match ir_kepler.wait_for_press().await {
            KeplerButton::Num(number) => {
                if number < PATTERNS.len() as u8 {
                    let pattern_index = number as usize;
                    conway.set_pattern_index(pattern_index);
                }
            }
            KeplerButton::Minus => {
                speed_mode = speed_mode.slower();
                conway.set_speed(speed_mode);
                info!("=== Speed: {:?} ===", speed_mode);
            }
            KeplerButton::Plus => {
                speed_mode = speed_mode.faster();
                conway.set_speed(speed_mode);
                info!("=== Speed: {:?} ===", speed_mode);
            }
            KeplerButton::Next => {
                conway.next_pattern();
            }
            KeplerButton::Prev => {
                conway.prev_pattern();
            }
            KeplerButton::PlayPause => {
                conway.toggle_pause();
            }
            KeplerButton::Mode => {
                conway.next_color();
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn conway_task(
    led16x16: Led16x16,
    signal: &'static Signal<CriticalSectionRawMutex, ConwayMessage>,
) {
    let mut board = Board::new();
    let mut pattern_index = 0;
    let mut speed_mode = SpeedMode::Slower;
    let mut paused = false;
    let mut color_index = 0usize;
    let mut alive_color = ALIVE_COLORS[color_index];
    board.add_pattern(PATTERNS[pattern_index]);

    // Track stasis for random mode: (generations without change, last live count)
    let mut stasis_tracker = (0u8, 0u16);
    let mut empty_tracker = 0u8;

    loop {
        let current_frame = board.to_frame(alive_color);
        led16x16.write_frame(current_frame).unwrap();

        // Calculate frame duration based on speed mode
        let frame_duration = match speed_mode {
            SpeedMode::Slower => Duration::from_millis(500),
            SpeedMode::Medium => Duration::from_millis(160),
            SpeedMode::Normal => Duration::from_millis(50),
        };

        // Race between timer and incoming message during steady frame display
        match select(Timer::after(frame_duration), signal.wait()).await {
            Either::First(_) => {
                // Timer fired, advance to next generation unless paused
                if paused {
                    continue;
                }
                board.step();

                let live_count = board.count_live_cells();
                let current_pattern = PATTERNS[pattern_index];

                if matches!(current_pattern, Pattern::Random | Pattern::Cross) {
                    let (unchanged_count, last_count) = stasis_tracker;

                    if live_count == last_count {
                        // Same count, increment counter
                        let new_unchanged_count = unchanged_count + 1;
                        stasis_tracker = (new_unchanged_count, live_count);

                        if new_unchanged_count >= 15 {
                            info!(
                                "Stasis detected ({} live cells for 15 generations), restarting pattern {:?}",
                                live_count, current_pattern
                            );

                            let mut new_board = Board::new();
                            new_board.add_pattern(current_pattern);

                            // Update to new board
                            board = new_board;
                            stasis_tracker = (0, 0);
                            empty_tracker = 0;
                        }
                    } else {
                        // Count changed, reset tracker
                        stasis_tracker = (1, live_count);
                    }
                } else if live_count == 0 {
                    empty_tracker += 1;
                    if empty_tracker >= 15 {
                        info!(
                            "Empty board detected for 15 generations, restarting pattern {:?}",
                            current_pattern
                        );

                        let mut new_board = Board::new();
                        new_board.add_pattern(current_pattern);

                        // Update to new board
                        board = new_board;
                        stasis_tracker = (0, 0);
                        empty_tracker = 0;
                    }
                } else {
                    empty_tracker = 0;
                }
            }
            Either::Second(msg) => {
                // Message received
                match msg {
                    ConwayMessage::NextPattern => {
                        if paused {
                            board.step();
                            let current_frame = board.to_frame(colors::LIME);
                            led16x16.write_frame(current_frame).unwrap();
                        } else {
                            // Pattern change requested
                            pattern_index = (pattern_index + 1) % PATTERNS.len();
                            let pattern = PATTERNS[pattern_index];
                            info!("=== Pattern: {:?} ===", pattern);

                            // Reset board with new pattern
                            board = Board::new();
                            board.add_pattern(pattern);

                            // Reset stasis detection
                            stasis_tracker = (0, 0);
                        }
                    }
                    ConwayMessage::PrevPattern => {
                        if paused {
                            continue;
                        }
                        pattern_index = (pattern_index + PATTERNS.len() - 1) % PATTERNS.len();
                        let pattern = PATTERNS[pattern_index];
                        info!("=== Pattern: {:?} ===", pattern);

                        // Reset board with new pattern
                        board = Board::new();
                        board.add_pattern(pattern);

                        // Reset stasis detection
                        stasis_tracker = (0, 0);
                        empty_tracker = 0;
                    }
                    ConwayMessage::SetSpeed(new_speed) => {
                        // Speed change requested
                        speed_mode = new_speed;
                    }
                    ConwayMessage::TogglePause => {
                        paused = !paused;
                        info!("=== {} ===", if paused { "Paused" } else { "Running" });
                    }
                    ConwayMessage::NextColor => {
                        color_index = (color_index + 1) % ALIVE_COLORS.len();
                        alive_color = ALIVE_COLORS[color_index];
                        info!("=== Color index: {} ===", color_index);
                        let current_frame = board.to_frame(alive_color);
                        led16x16.write_frame(current_frame).unwrap();
                    }
                    ConwayMessage::SetPatternIndex(new_pattern_index) => {
                        assert!(new_pattern_index < PATTERNS.len());
                        pattern_index = new_pattern_index;
                        let pattern = PATTERNS[pattern_index];
                        info!("=== Pattern: {:?} ===", pattern);

                        // Reset board with new pattern
                        board = Board::new();
                        board.add_pattern(pattern);

                        // Reset stasis detection
                        stasis_tracker = (0, 0);
                        empty_tracker = 0;
                    }
                }
            }
        }
    }
}

/// Conway's Game of Life board with toroidal wrapping.
#[derive(Copy, Clone)]
struct Board<const H: usize, const W: usize> {
    cells: [[bool; W]; H],
}

impl<const H: usize, const W: usize> Board<H, W> {
    /// Create a new empty board.
    fn new() -> Self {
        Self {
            cells: [[false; W]; H],
        }
    }

    /// Initialize board with a pattern.
    fn add_pattern(&mut self, pattern: Pattern) {
        match pattern {
            Pattern::Glider => self.add_glider(4, 2),
            Pattern::Blinker => self.add_blinker(5, 4),
            Pattern::Toad => self.add_toad(5, 4),
            Pattern::Beacon => self.add_beacon(4, 4),
            Pattern::LWSS => self.add_lwss(5, 6),
            Pattern::Block => self.add_block(5, 4),
            Pattern::Wall => self.add_wall(5),
            Pattern::Random => self.add_random(),
            Pattern::Cross => self.add_cross(7, 7),
        }
    }

    /// Glider pattern (moves diagonally).
    fn add_glider(&mut self, start_row: usize, start_col: usize) {
        self.cells[start_row][start_col + 1] = true;
        self.cells[start_row + 1][start_col + 2] = true;
        self.cells[start_row + 2][start_col] = true;
        self.cells[start_row + 2][start_col + 1] = true;
        self.cells[start_row + 2][start_col + 2] = true;
    }

    /// Blinker pattern (period 2 oscillator, 3 cells).
    fn add_blinker(&mut self, row: usize, col: usize) {
        self.cells[row][col] = true;
        self.cells[row][col + 1] = true;
        self.cells[row][col + 2] = true;
    }

    /// Toad pattern (period 2 oscillator, 6 cells).
    fn add_toad(&mut self, row: usize, col: usize) {
        self.cells[row][col + 1] = true;
        self.cells[row][col + 2] = true;
        self.cells[row][col + 3] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 1][col + 1] = true;
        self.cells[row + 1][col + 2] = true;
    }

    /// Beacon pattern (period 2 oscillator, 4 cells in corners).
    fn add_beacon(&mut self, row: usize, col: usize) {
        self.cells[row][col] = true;
        self.cells[row][col + 1] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 1][col + 1] = true;
        self.cells[row + 2][col + 2] = true;
        self.cells[row + 2][col + 3] = true;
        self.cells[row + 3][col + 2] = true;
        self.cells[row + 3][col + 3] = true;
    }

    /// Lightweight Spaceship (LWSS) - moves horizontally.
    fn add_lwss(&mut self, row: usize, col: usize) {
        self.cells[row][col + 1] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 2][col] = true;
        self.cells[row + 2][col + 1] = true;
        self.cells[row + 2][col + 2] = true;
        self.cells[row + 2][col + 3] = true;
        self.cells[row + 1][col + 3] = true;
    }

    /// Block pattern (stable 2×2 square).
    fn add_block(&mut self, row: usize, col: usize) {
        self.cells[row][col] = true;
        self.cells[row][col + 1] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 1][col + 1] = true;
    }

    /// Horizontal wall (full-width line).
    fn add_wall(&mut self, row: usize) {
        for x_index in 0..W {
            self.cells[row][x_index] = true;
        }
    }

    /// Vertical wall (full-height line).
    fn add_vertical(&mut self, col: usize) {
        for y_index in 0..H {
            self.cells[y_index][col] = true;
        }
    }

    /// Cross: horizontal + vertical lines.
    fn add_cross(&mut self, row: usize, col: usize) {
        self.add_wall(row);
        self.add_vertical(col);
    }

    /// Random pattern seeded by time.
    fn add_random(&mut self) {
        let now = embassy_time::Instant::now().as_millis();
        // Simple LCG based on current time
        let mut seed = (now ^ 0x9e37_79b9) as u32;
        for y_index in 0..H {
            for x_index in 0..W {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.cells[y_index][x_index] = (seed & 0x100) != 0;
            }
        }
    }

    /// Compute the next generation in place.
    fn step(&mut self) {
        let mut next_cells = [[false; W]; H];

        for y_index in 0..H {
            for x_index in 0..W {
                let live_neighbors = self.count_live_neighbors(y_index, x_index);
                let is_alive = self.cells[y_index][x_index];

                // Conway's Game of Life rules:
                // 1. Any live cell with 2 or 3 live neighbors survives
                // 2. Any dead cell with exactly 3 live neighbors becomes alive
                // 3. All other cells die or stay dead
                next_cells[y_index][x_index] = match (is_alive, live_neighbors) {
                    (true, 2) | (true, 3) => true,
                    (false, 3) => true,
                    _ => false,
                };
            }
        }

        self.cells = next_cells;
    }

    /// Count the number of live neighbors for a cell at (row, col).
    /// Wraps around board edges (toroidal topology).
    fn count_live_neighbors(&self, row: usize, col: usize) -> u8 {
        let mut count = 0u8;

        // Check all 8 neighbors with wrapping
        for row_offset in [-1, 0, 1].iter().copied() {
            for col_offset in [-1, 0, 1].iter().copied() {
                // Skip the center cell
                if row_offset == 0 && col_offset == 0 {
                    continue;
                }

                // Wrap coordinates around board edges
                let neighbor_row = ((row as isize + row_offset).rem_euclid(H as isize)) as usize;
                let neighbor_col = ((col as isize + col_offset).rem_euclid(W as isize)) as usize;

                if self.cells[neighbor_row][neighbor_col] {
                    count += 1;
                }
            }
        }

        count
    }

    /// Count the total number of live cells on the board.
    fn count_live_cells(&self) -> u16 {
        let mut count = 0u16;
        for row in &self.cells {
            for &cell in row {
                if cell {
                    count += 1;
                }
            }
        }
        count
    }

    /// Convert board state to an LED frame with the specified color for alive cells.
    fn to_frame(&self, alive_color: RGB8) -> Frame2d<W, H> {
        let mut frame = Frame2d::<W, H>::new();
        for y_index in 0..H {
            for x_index in 0..W {
                if self.cells[y_index][x_index] {
                    frame[(x_index, y_index)] = alive_color;
                }
            }
        }
        frame
    }
}

/// Static resources for Conway's Game of Life device.
struct ConwayStatic {
    signal: Signal<CriticalSectionRawMutex, ConwayMessage>,
}

impl ConwayStatic {
    /// Create static resources for Conway.
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }
}

/// Conway's Game of Life device that runs in its own spawned task.
/// Wraps a reference to the static signal for sending pattern change messages.
struct Conway<'a>(&'a Signal<CriticalSectionRawMutex, ConwayMessage>);

impl Conway<'_> {
    /// Create static resources for Conway.
    #[must_use]
    pub const fn new_static() -> ConwayStatic {
        ConwayStatic::new()
    }

    /// Create a new Conway device, spawn its background task, and return a handle for control.
    pub fn new(
        conway_static: &'static ConwayStatic,
        led16x16: Led16x16,
        spawner: Spawner,
    ) -> Result<Self> {
        let token = conway_task(led16x16, &conway_static.signal);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(&conway_static.signal))
    }

    /// Send a message to request the next pattern.
    pub fn next_pattern(&self) {
        self.0.signal(ConwayMessage::NextPattern);
    }

    /// Send a message to request the previous pattern.
    pub fn prev_pattern(&self) {
        self.0.signal(ConwayMessage::PrevPattern);
    }

    /// Send a message to change the simulation speed.
    pub fn set_speed(&self, speed: SpeedMode) {
        self.0.signal(ConwayMessage::SetSpeed(speed));
    }

    /// Send a message to select a specific pattern by index.
    pub fn set_pattern_index(&self, pattern_index: usize) {
        self.0.signal(ConwayMessage::SetPatternIndex(pattern_index));
    }

    /// Send a message to toggle pause/resume.
    pub fn toggle_pause(&self) {
        self.0.signal(ConwayMessage::TogglePause);
    }

    /// Send a message to advance the alive-cell color.
    pub fn next_color(&self) {
        self.0.signal(ConwayMessage::NextColor);
    }
}
