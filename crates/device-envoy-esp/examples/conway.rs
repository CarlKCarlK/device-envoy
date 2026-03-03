#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    esp_hal::gpio::{Level, Output, OutputConfig},
    init_and_start,
    ir::{IrKepler, IrKeplerStatic, KeplerButton},
    led2d,
    led2d::{layout::LedLayout, Frame2d, Led2dFont},
    led_strip::{colors, Current, RGB8},
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_16X16: LedLayout<256, 16, 16> = LedLayout::serpentine_row_major();
const PANEL_16X16_PIN_NUM: u8 = 2;
const IR_PIN_NUM: u8 = 7;

led2d! {
    Led16x16Conway {
        len: 256,
        led_layout: LED_LAYOUT_16X16,
        max_current: Current::Milliamps(700),
        font: Led2dFont::Font4x6Trim,
        engine: Engine::Spi,
        max_frames: 30,
    }
}

#[derive(Clone, Copy, Debug)]
enum ConwayMessage {
    NextPattern,
    PrevPattern,
    SetSpeed(SpeedMode),
    SetPatternIndex(usize),
    TogglePause,
    NextColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeedMode {
    Slower,
    Medium,
    Normal,
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

const PATTERNS: &[Pattern] = &[
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

const ALIVE_COLORS: &[RGB8] = &[
    colors::LIME,
    colors::CYAN,
    colors::MAGENTA,
    colors::ORANGE,
    colors::YELLOW,
    colors::WHITE,
];

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!(
        "Conway: 16x16 SPI on GPIO{}, IR receiver on GPIO{}",
        PANEL_16X16_PIN_NUM, IR_PIN_NUM
    );

    // TODO0 Keep MAX98357A I2S inputs quiet in this non-audio example.
    let _audio_idle_pins = (
        Output::new(p.GPIO21, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO11, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO12, Level::Low, OutputConfig::default()),
    );

    let led16x16_conway = Led16x16Conway::new(p.GPIO2, p.SPI2, spawner)?;
    static IR_KEPLER_STATIC: IrKeplerStatic = IrKepler::new_static();
    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let ir_rmt_channel = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let ir_rmt_channel = rmt80.channel2;
    let ir_kepler = IrKepler::new(&IR_KEPLER_STATIC, p.GPIO7, ir_rmt_channel, spawner)?;

    static CONWAY_STATIC: ConwayStatic = Conway::new_static();
    let conway = Conway::new(&CONWAY_STATIC, led16x16_conway, spawner)?;

    let mut speed_mode = SpeedMode::Slower;
    loop {
        match ir_kepler.wait_for_press().await {
            KeplerButton::Num(number) => {
                if number < PATTERNS.len() as u8 {
                    conway.set_pattern_index(number as usize);
                }
            }
            KeplerButton::Minus => {
                speed_mode = speed_mode.slower();
                conway.set_speed(speed_mode);
                info!("speed: {:?}", speed_mode);
            }
            KeplerButton::Plus => {
                speed_mode = speed_mode.faster();
                conway.set_speed(speed_mode);
                info!("speed: {:?}", speed_mode);
            }
            KeplerButton::Next => conway.next_pattern(),
            KeplerButton::Prev => conway.prev_pattern(),
            KeplerButton::PlayPause => conway.toggle_pause(),
            KeplerButton::Mode => conway.next_color(),
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn conway_task(
    led16x16_conway: &'static Led16x16Conway,
    signal: &'static Signal<CriticalSectionRawMutex, ConwayMessage>,
) -> ! {
    let mut board = Board::new();
    let mut pattern_index = 0usize;
    let mut speed_mode = SpeedMode::Slower;
    let mut paused = false;
    let mut color_index = 0usize;
    let mut alive_color = ALIVE_COLORS[color_index];
    board.add_pattern(PATTERNS[pattern_index]);

    let mut stasis_tracker = (0u8, 0u16);
    let mut empty_tracker = 0u8;

    loop {
        let current_frame = board.to_frame(alive_color);
        led16x16_conway.write_frame2d(current_frame);

        let frame_duration = match speed_mode {
            SpeedMode::Slower => Duration::from_millis(500),
            SpeedMode::Medium => Duration::from_millis(160),
            SpeedMode::Normal => Duration::from_millis(50),
        };

        match select(Timer::after(frame_duration), signal.wait()).await {
            Either::First(_) => {
                if paused {
                    continue;
                }
                board.step();

                let live_cell_count = board.count_live_cells();
                let current_pattern = PATTERNS[pattern_index];

                if matches!(current_pattern, Pattern::Random | Pattern::Cross) {
                    let (unchanged_count, last_live_count) = stasis_tracker;
                    if live_cell_count == last_live_count {
                        let new_unchanged_count = unchanged_count + 1;
                        stasis_tracker = (new_unchanged_count, live_cell_count);
                        if new_unchanged_count >= 15 {
                            info!(
                                "stasis: {} live cells for 15 generations; restart {:?}",
                                live_cell_count, current_pattern
                            );
                            let mut next_board = Board::new();
                            next_board.add_pattern(current_pattern);
                            board = next_board;
                            stasis_tracker = (0, 0);
                            empty_tracker = 0;
                        }
                    } else {
                        stasis_tracker = (1, live_cell_count);
                    }
                } else if live_cell_count == 0 {
                    empty_tracker += 1;
                    if empty_tracker >= 15 {
                        info!(
                            "empty board for 15 generations; restart {:?}",
                            current_pattern
                        );
                        let mut next_board = Board::new();
                        next_board.add_pattern(current_pattern);
                        board = next_board;
                        stasis_tracker = (0, 0);
                        empty_tracker = 0;
                    }
                } else {
                    empty_tracker = 0;
                }
            }
            Either::Second(msg) => match msg {
                ConwayMessage::NextPattern => {
                    if paused {
                        board.step();
                        let frame2d = board.to_frame(alive_color);
                        led16x16_conway.write_frame2d(frame2d);
                    } else {
                        pattern_index = (pattern_index + 1) % PATTERNS.len();
                        let pattern = PATTERNS[pattern_index];
                        info!("pattern: {:?}", pattern);
                        board = Board::new();
                        board.add_pattern(pattern);
                        stasis_tracker = (0, 0);
                        empty_tracker = 0;
                    }
                }
                ConwayMessage::PrevPattern => {
                    if paused {
                        continue;
                    }
                    pattern_index = (pattern_index + PATTERNS.len() - 1) % PATTERNS.len();
                    let pattern = PATTERNS[pattern_index];
                    info!("pattern: {:?}", pattern);
                    board = Board::new();
                    board.add_pattern(pattern);
                    stasis_tracker = (0, 0);
                    empty_tracker = 0;
                }
                ConwayMessage::SetSpeed(new_speed_mode) => {
                    speed_mode = new_speed_mode;
                }
                ConwayMessage::TogglePause => {
                    paused = !paused;
                    info!("{}", if paused { "paused" } else { "running" });
                }
                ConwayMessage::NextColor => {
                    color_index = (color_index + 1) % ALIVE_COLORS.len();
                    alive_color = ALIVE_COLORS[color_index];
                    info!("color index: {}", color_index);
                    let frame2d = board.to_frame(alive_color);
                    led16x16_conway.write_frame2d(frame2d);
                }
                ConwayMessage::SetPatternIndex(new_pattern_index) => {
                    assert!(new_pattern_index < PATTERNS.len());
                    pattern_index = new_pattern_index;
                    let pattern = PATTERNS[pattern_index];
                    info!("pattern: {:?}", pattern);
                    board = Board::new();
                    board.add_pattern(pattern);
                    stasis_tracker = (0, 0);
                    empty_tracker = 0;
                }
            },
        }
    }
}

include!("data/conway_board.rs");

impl Board<16, 16> {
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

    fn add_glider(&mut self, start_row: usize, start_col: usize) {
        self.cells[start_row][start_col + 1] = true;
        self.cells[start_row + 1][start_col + 2] = true;
        self.cells[start_row + 2][start_col] = true;
        self.cells[start_row + 2][start_col + 1] = true;
        self.cells[start_row + 2][start_col + 2] = true;
    }

    fn add_blinker(&mut self, row: usize, col: usize) {
        self.cells[row][col] = true;
        self.cells[row][col + 1] = true;
        self.cells[row][col + 2] = true;
    }

    fn add_toad(&mut self, row: usize, col: usize) {
        self.cells[row][col + 1] = true;
        self.cells[row][col + 2] = true;
        self.cells[row][col + 3] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 1][col + 1] = true;
        self.cells[row + 1][col + 2] = true;
    }

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

    fn add_lwss(&mut self, row: usize, col: usize) {
        self.cells[row][col + 1] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 2][col] = true;
        self.cells[row + 2][col + 1] = true;
        self.cells[row + 2][col + 2] = true;
        self.cells[row + 2][col + 3] = true;
        self.cells[row + 1][col + 3] = true;
    }

    fn add_block(&mut self, row: usize, col: usize) {
        self.cells[row][col] = true;
        self.cells[row][col + 1] = true;
        self.cells[row + 1][col] = true;
        self.cells[row + 1][col + 1] = true;
    }

    fn add_wall(&mut self, row: usize) {
        for x_index in 0..16 {
            self.cells[row][x_index] = true;
        }
    }

    fn add_vertical(&mut self, col: usize) {
        for y_index in 0..16 {
            self.cells[y_index][col] = true;
        }
    }

    fn add_cross(&mut self, row: usize, col: usize) {
        self.add_wall(row);
        self.add_vertical(col);
    }

    fn add_random(&mut self) {
        let now_millis = embassy_time::Instant::now().as_millis();
        let mut seed = (now_millis ^ 0x9e37_79b9) as u32;
        for y_index in 0..16 {
            for x_index in 0..16 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.cells[y_index][x_index] = (seed & 0x100) != 0;
            }
        }
    }

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

    fn add_pentadecathlon(&mut self) {
        self.load_rows([
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
        ]);
    }

    fn add_custom9(&mut self) {
        self.load_rows([
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
        ]);
    }
}

struct ConwayStatic {
    signal: Signal<CriticalSectionRawMutex, ConwayMessage>,
}

impl ConwayStatic {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }
}

struct Conway<'a>(&'a Signal<CriticalSectionRawMutex, ConwayMessage>);

impl Conway<'_> {
    const fn new_static() -> ConwayStatic {
        ConwayStatic::new()
    }

    fn new(
        conway_static: &'static ConwayStatic,
        led16x16_conway: &'static Led16x16Conway,
        spawner: Spawner,
    ) -> device_envoy_esp::Result<Self> {
        spawner
            .spawn(conway_task(led16x16_conway, &conway_static.signal))
            .map_err(device_envoy_esp::Error::TaskSpawn)?;
        Ok(Self(&conway_static.signal))
    }

    fn next_pattern(&self) {
        self.0.signal(ConwayMessage::NextPattern);
    }

    fn prev_pattern(&self) {
        self.0.signal(ConwayMessage::PrevPattern);
    }

    fn set_speed(&self, speed_mode: SpeedMode) {
        self.0.signal(ConwayMessage::SetSpeed(speed_mode));
    }

    fn set_pattern_index(&self, pattern_index: usize) {
        self.0.signal(ConwayMessage::SetPatternIndex(pattern_index));
    }

    fn toggle_pause(&self) {
        self.0.signal(ConwayMessage::TogglePause);
    }

    fn next_color(&self) {
        self.0.signal(ConwayMessage::NextColor);
    }
}
