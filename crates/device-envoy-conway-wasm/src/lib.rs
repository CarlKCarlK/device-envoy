use device_envoy_conway_core::{Board, Pattern, PredecessorSearch, SearchOutcome, SearchStep};
use smart_leds::{colors, RGB8};
use wasm_bindgen::prelude::*;

const WIDTH: usize = 16;
const HEIGHT: usize = 16;
const DEFAULT_MAX_DIMENSION: u32 = 640;
const SEARCH_ITERATIONS_PER_COMMAND: u32 = 256;

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

#[derive(Clone, Copy)]
enum SpeedMode {
    Slow,
    Medium,
    Fast,
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
    const fn interval_ms(self) -> u32 {
        match self {
            Self::Slow => 500,
            Self::Medium => 160,
            Self::Fast => 50,
        }
    }
}

#[wasm_bindgen]
pub struct ConwayWeb {
    board: Board<HEIGHT, WIDTH>,
    pattern_index: usize,
    search: Option<PredecessorSearch<HEIGHT, WIDTH>>,
    paused: bool,
    speed_mode: SpeedMode,
    color_index: usize,
}

#[wasm_bindgen]
impl ConwayWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let pattern_index = 1usize;
        let color_index = 1usize; // CYAN, matching embedded default
        let mut board = Board::new();
        board.add_pattern(PATTERNS[pattern_index]);
        Self {
            board,
            pattern_index,
            search: None,
            paused: false,
            speed_mode: SpeedMode::Medium,
            color_index,
        }
    }

    pub fn tick_interval_ms(&self) -> u32 {
        self.speed_mode.interval_ms()
    }

    pub fn press_key(&mut self, key: &str) -> String {
        match key {
            "prev" => {
                self.search = Some(PredecessorSearch::new(self.board));
                "searching".into()
            }
            "next" => {
                if self.paused {
                    self.board.step();
                }
                "ok".into()
            }
            "play_pause" => {
                self.paused = !self.paused;
                "ok".into()
            }
            "cancel" => {
                self.search = None;
                "cancelled".into()
            }
            "speed_up" => {
                self.speed_mode = self.speed_mode.faster();
                "ok".into()
            }
            "speed_down" => {
                self.speed_mode = self.speed_mode.slower();
                "ok".into()
            }
            "mode" => {
                self.color_index = (self.color_index + 1) % ALIVE_COLORS.len();
                "ok".into()
            }
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let pattern_index = key.as_bytes()[0] - b'0';
                self.set_pattern(pattern_index as usize);
                "ok".into()
            }
            _ => "unknown".into(),
        }
    }

    pub fn tick(&mut self) -> String {
        if let Some(search) = &mut self.search {
            match search.advance(SEARCH_ITERATIONS_PER_COMMAND) {
                SearchStep::Progress { .. } => "searching".into(),
                SearchStep::Outcome(SearchOutcome::Found(predecessor)) => {
                    self.board = predecessor;
                    self.search = None;
                    "found".into()
                }
                SearchStep::Outcome(SearchOutcome::NotFound) => {
                    self.search = None;
                    "not_found".into()
                }
                SearchStep::Outcome(SearchOutcome::Cancelled) => {
                    self.search = None;
                    "cancelled".into()
                }
            }
        } else if self.paused {
            "paused".into()
        } else {
            self.board.step();
            "ok".into()
        }
    }

    pub fn render_png(&mut self) -> Result<Vec<u8>, JsValue> {
        self.render_png_with_max_dimension(DEFAULT_MAX_DIMENSION)
    }

    pub fn render_png_with_max_dimension(
        &mut self,
        max_dimension: u32,
    ) -> Result<Vec<u8>, JsValue> {
        let alive_color = ALIVE_COLORS[self.color_index];
        let frame = if self.search.is_some() {
            match self.search_preview() {
                Some(frame) => frame,
                None => self.board.to_frame(alive_color),
            }
        } else {
            self.board.to_frame(alive_color)
        };
        frame
            .to_png_bytes(max_dimension)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

impl Default for ConwayWeb {
    fn default() -> Self {
        Self::new()
    }
}

impl ConwayWeb {
    fn set_pattern(&mut self, pattern_index: usize) {
        if pattern_index < PATTERNS.len() {
            self.pattern_index = pattern_index;
            self.board = Board::new();
            self.board.add_pattern(PATTERNS[pattern_index]);
            self.search = None;
        }
    }
    fn search_preview(&mut self) -> Option<device_envoy_core::led2d::Frame2d<WIDTH, HEIGHT>> {
        let search = self.search.as_mut()?;
        match search.advance(1) {
            SearchStep::Progress {
                candidate,
                assigned,
                target,
            } => Some(search_frame(&candidate, &assigned, &target)),
            SearchStep::Outcome(_) => None,
        }
    }
}

fn search_frame(
    candidate: &Board<HEIGHT, WIDTH>,
    assigned: &[[bool; WIDTH]; HEIGHT],
    target: &Board<HEIGHT, WIDTH>,
) -> device_envoy_core::led2d::Frame2d<WIDTH, HEIGHT> {
    let mut frame = device_envoy_core::led2d::Frame2d::<WIDTH, HEIGHT>::new();
    for row_index in 0..HEIGHT {
        for col_index in 0..WIDTH {
            if target.cells[row_index][col_index] {
                frame[(col_index, row_index)] = smart_leds::RGB8 { r: 0, g: 10, b: 0 };
            }

            if assigned[row_index][col_index] {
                frame[(col_index, row_index)] = if candidate.cells[row_index][col_index] {
                    colors::RED
                } else {
                    smart_leds::RGB8 { r: 0, g: 0, b: 12 }
                };
            }
        }
    }
    frame
}
