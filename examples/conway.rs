#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_kit::{Error, Result};
use device_kit::button::{Button, PressDuration, PressedTo};
use device_kit::led_strip::Current;
use device_kit::led_strip::Gamma;
use device_kit::led_strip::Rgb;
use device_kit::led2d;
use device_kit::led2d::Frame2d;
use device_kit::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_rp::init;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use panic_probe as _;
use smart_leds::colors;

// Two 12x4 panels stacked vertically and rotated 90° CW → 8×12 display.
const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_8X12: LedLayout<96, 8, 12> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4).rotate_cw();

// cmk000 add default
led2d! {
    pub Led8x12,
    pio: PIO0,
    pin: PIN_4,
    dma: DMA_CH0,
    width: 8,
    height: 12,
    led_layout: LED_LAYOUT_8X12,
    max_current: Current::Milliamps(1000),
    gamma: Gamma::Linear,
    max_frames: 32,
    font: Font4x6Trim,
}

/// Message type for communicating pattern changes and speed adjustments to the Conway task.
#[derive(Clone, Copy, Debug, defmt::Format)]
enum ConwayMessage {
    NextPattern,
    SetSpeed(SpeedMode),
}

/// Speed modes for the simulation.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
enum SpeedMode {
    Slower, // 10x slower (500ms per generation)
    Normal, // 1x normal (50ms per generation)
    Faster, // 10x faster (5ms per generation)
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
}

const PATTERNS: &[Pattern] = &[
    Pattern::Glider,
    Pattern::Blinker,
    Pattern::Toad,
    Pattern::Beacon,
    Pattern::LWSS,
    Pattern::Block,
    Pattern::Wall,
    Pattern::Random,
];

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Conway's Game of Life on 8x12 LED board");
    let p = init(Default::default());

    let led8x12 = Led8x12::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;
    let mut button = Button::new(p.PIN_13, PressedTo::Ground);

    // Create Conway device with static resources and spawn background task
    static CONWAY_STATIC: ConwayStatic = Conway::new_static();
    let conway = Conway::new(&CONWAY_STATIC, led8x12, spawner)?;

    // Speed mode cycling state
    let mut speed_mode = SpeedMode::Slower;

    // Main loop: detect button presses and long-presses
    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                conway.next_pattern();
            }
            PressDuration::Long => {
                speed_mode = match speed_mode {
                    SpeedMode::Slower => SpeedMode::Normal,
                    SpeedMode::Normal => SpeedMode::Faster,
                    SpeedMode::Faster => SpeedMode::Slower,
                };
                conway.set_speed(speed_mode);
                info!("=== Speed: {:?} ===", speed_mode);
            }
        }
    }
}

#[embassy_executor::task]
async fn conway_task(
    led8x12: Led8x12,
    signal: &'static Signal<CriticalSectionRawMutex, ConwayMessage>,
) {
    let mut board = Board::<{ Led8x12::HEIGHT }, { Led8x12::WIDTH }>::new();
    let mut pattern_index = 0;
    let mut speed_mode = SpeedMode::Slower;
    board.add_pattern(PATTERNS[pattern_index]);

    loop {
        let frame = board.to_frame(colors::GREEN);
        led8x12
            .write_frame(frame)
            .await
            .expect("write_frame failed");

        // Calculate frame duration based on speed mode
        let frame_duration = match speed_mode {
            SpeedMode::Slower => Duration::from_millis(500),
            SpeedMode::Normal => Duration::from_millis(50),
            SpeedMode::Faster => Duration::from_millis(5),
        };

        // Race between timer and incoming message
        match select(Timer::after(frame_duration), signal.wait()).await {
            Either::First(_) => {
                // Timer fired, advance generation
                board.step();
            }
            Either::Second(msg) => {
                // Message received
                match msg {
                    ConwayMessage::NextPattern => {
                        // Pattern change requested
                        pattern_index = (pattern_index + 1) % PATTERNS.len();
                        let pattern = PATTERNS[pattern_index];
                        info!("=== Pattern: {:?} ===", pattern);

                        // Reset board with new pattern
                        board = Board::<{ Led8x12::HEIGHT }, { Led8x12::WIDTH }>::new();
                        board.add_pattern(pattern);
                    }
                    ConwayMessage::SetSpeed(new_speed) => {
                        // Speed change requested
                        speed_mode = new_speed;
                    }
                }
            }
        }
    }
}

/// Conway's Game of Life board with toroidal wrapping.
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
            Pattern::LWSS => self.add_lwss(5, 2),
            Pattern::Block => self.add_block(5, 4),
            Pattern::Wall => self.add_wall(5),
            Pattern::Random => self.add_random(),
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
        for col_index in 0..W {
            self.cells[row][col_index] = true;
        }
    }

    /// Random pattern seeded by time.
    fn add_random(&mut self) {
        let now = embassy_time::Instant::now().as_millis();
        // Simple LCG based on current time
        let mut seed = (now ^ 0x9e37_79b9) as u32;
        for row_index in 0..H {
            for col_index in 0..W {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.cells[row_index][col_index] = (seed & 0x100) != 0;
            }
        }
    }

    /// Compute the next generation in place.
    fn step(&mut self) {
        let mut next_cells = [[false; W]; H];

        for row_index in 0..H {
            for col_index in 0..W {
                let live_neighbors = self.count_live_neighbors(row_index, col_index);
                let is_alive = self.cells[row_index][col_index];

                // Conway's Game of Life rules:
                // 1. Any live cell with 2 or 3 live neighbors survives
                // 2. Any dead cell with exactly 3 live neighbors becomes alive
                // 3. All other cells die or stay dead
                next_cells[row_index][col_index] = match (is_alive, live_neighbors) {
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

    /// Convert board state to an LED frame with the specified color for alive cells.
    fn to_frame(&self, alive_color: Rgb) -> Frame2d<8, 12> {
        let mut frame = Frame2d::<8, 12>::new();
        for row_index in 0..H {
            for col_index in 0..W {
                if self.cells[row_index][col_index] {
                    frame[row_index][col_index] = alive_color;
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

// cmk make generic for any size led2d
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
        led8x12: Led8x12,
        spawner: Spawner,
    ) -> Result<Self> {
        let token = conway_task(led8x12, &conway_static.signal);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(&conway_static.signal))
    }

    /// Send a message to request the next pattern.
    pub fn next_pattern(&self) {
        self.0.signal(ConwayMessage::NextPattern);
    }

    /// Send a message to change the simulation speed.
    pub fn set_speed(&self, speed: SpeedMode) {
        self.0.signal(ConwayMessage::SetSpeed(speed));
    }
}
