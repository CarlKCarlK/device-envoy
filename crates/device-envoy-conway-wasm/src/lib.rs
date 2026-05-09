use device_envoy_conway_core::{Board, Pattern, PredecessorSearch, SearchOutcome, SearchStep};
use smart_leds::colors;
use wasm_bindgen::prelude::*;

const WIDTH: usize = 16;
const HEIGHT: usize = 16;
const DEFAULT_MAX_DIMENSION: u32 = 640;
const SEARCH_ITERATIONS_PER_COMMAND: u32 = 256;
const STASIS_RESET_GENERATIONS: u8 = 15;
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

#[wasm_bindgen]
pub struct ConwayWeb {
    board: Board<HEIGHT, WIDTH>,
    pattern_index: usize,
    random_symmetry_mode: RandomSymmetryMode,
    search: Option<PredecessorSearch<HEIGHT, WIDTH>>,
    paused: bool,
    speed_mode: SpeedMode,
    unchanged_live_count_generations: u8,
    unchanged_board_generations: u8,
    last_live_count: u16,
    previous_board: Board<HEIGHT, WIDTH>,
    empty_tracker: u8,
    random_seed_state: u32,
}

#[wasm_bindgen]
impl ConwayWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut board = Board::new();
        let pattern_index = 1usize;
        let random_symmetry_mode = RandomSymmetryMode::None;
        add_pattern(
            &mut board,
            PATTERNS[pattern_index],
            random_symmetry_mode,
            0x9e37_79b9,
        );
        let last_live_count = board.count_live_cells();
        Self {
            board,
            pattern_index,
            random_symmetry_mode,
            search: None,
            paused: false,
            speed_mode: SpeedMode::Medium,
            unchanged_live_count_generations: 0,
            unchanged_board_generations: 0,
            last_live_count,
            previous_board: board,
            empty_tracker: 0,
            random_seed_state: 0x9e37_79b9,
        }
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
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let pattern_index = key.as_bytes()[0] - b'0';
                self.set_pattern(pattern_index as usize);
                "ok".into()
            }
            "repeat" => {
                if PATTERNS[self.pattern_index] == Pattern::Random {
                    self.random_symmetry_mode = self.random_symmetry_mode.next();
                    self.reseed_current_pattern();
                    self.search = None;
                    self.unchanged_live_count_generations = 0;
                    self.unchanged_board_generations = 0;
                    self.last_live_count = self.board.count_live_cells();
                    self.previous_board = self.board;
                    self.empty_tracker = 0;
                }
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
                    self.unchanged_live_count_generations = 0;
                    self.unchanged_board_generations = 0;
                    self.last_live_count = self.board.count_live_cells();
                    self.previous_board = self.board;
                    self.empty_tracker = 0;
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
            self.previous_board = self.board;
            self.board.step();
            self.evaluate_auto_reset();
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
        let frame = if self.search.is_some() {
            match self.search_preview() {
                Some(frame) => frame,
                None => self.board.to_frame(colors::CYAN),
            }
        } else {
            self.board.to_frame(colors::CYAN)
        };
        frame
            .to_png_bytes(max_dimension)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    pub fn tick_interval_ms(&self) -> u32 {
        self.speed_mode.frame_duration_ms()
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
            if PATTERNS[pattern_index] == Pattern::Random {
                self.random_symmetry_mode = RandomSymmetryMode::None;
            }
            self.reseed_current_pattern();
            self.search = None;
            self.unchanged_live_count_generations = 0;
            self.unchanged_board_generations = 0;
            self.last_live_count = self.board.count_live_cells();
            self.previous_board = self.board;
            self.empty_tracker = 0;
        }
    }

    fn reseed_current_pattern(&mut self) {
        self.random_seed_state = self
            .random_seed_state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.board = Board::new();
        add_pattern(
            &mut self.board,
            PATTERNS[self.pattern_index],
            self.random_symmetry_mode,
            self.random_seed_state,
        );
    }

    fn evaluate_auto_reset(&mut self) {
        let live_cell_count = self.board.count_live_cells();
        let current_pattern = PATTERNS[self.pattern_index];

        if matches!(current_pattern, Pattern::Random | Pattern::Cross) {
            if live_cell_count == self.last_live_count {
                self.unchanged_live_count_generations =
                    self.unchanged_live_count_generations.saturating_add(1);
            } else {
                self.unchanged_live_count_generations = 0;
            }
            self.last_live_count = live_cell_count;

            if self.board == self.previous_board {
                self.unchanged_board_generations =
                    self.unchanged_board_generations.saturating_add(1);
            } else {
                self.unchanged_board_generations = 0;
            }

            if self.unchanged_live_count_generations >= STASIS_RESET_GENERATIONS
                || self.unchanged_board_generations >= STASIS_RESET_GENERATIONS
            {
                    self.board = Board::new();
                    self.reseed_current_pattern();
                    self.unchanged_live_count_generations = 0;
                    self.unchanged_board_generations = 0;
                    self.last_live_count = self.board.count_live_cells();
                    self.previous_board = self.board;
                    self.empty_tracker = 0;
            }
        } else if live_cell_count == 0 {
            self.empty_tracker += 1;
            if self.empty_tracker >= STASIS_RESET_GENERATIONS {
                self.board = Board::new();
                self.reseed_current_pattern();
                self.unchanged_live_count_generations = 0;
                self.unchanged_board_generations = 0;
                self.last_live_count = self.board.count_live_cells();
                self.previous_board = self.board;
                self.empty_tracker = 0;
            }
        } else {
            self.empty_tracker = 0;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RandomSymmetryMode {
    None,
    LeftRightNoCenter,
    LeftRightCentered,
    FourWayNoCenter,
    FourWayCentered,
    DiagonalNoCenter,
    DiagonalCentered,
    DiagonalFourWayNoCenter,
    DiagonalFourWayCentered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    const fn frame_duration_ms(self) -> u32 {
        match self {
            Self::Slow => 500,
            Self::Medium => 160,
            Self::Fast => 50,
        }
    }
}

impl RandomSymmetryMode {
    const fn next(self) -> Self {
        match self {
            Self::None => Self::LeftRightNoCenter,
            Self::LeftRightNoCenter => Self::LeftRightCentered,
            Self::LeftRightCentered => Self::FourWayNoCenter,
            Self::FourWayNoCenter => Self::FourWayCentered,
            Self::FourWayCentered => Self::DiagonalNoCenter,
            Self::DiagonalNoCenter => Self::DiagonalCentered,
            Self::DiagonalCentered => Self::DiagonalFourWayNoCenter,
            Self::DiagonalFourWayNoCenter => Self::DiagonalFourWayCentered,
            Self::DiagonalFourWayCentered => Self::None,
        }
    }
}

fn add_pattern(
    board: &mut Board<HEIGHT, WIDTH>,
    pattern: Pattern,
    random_symmetry_mode: RandomSymmetryMode,
    random_seed: u32,
) {
    if pattern == Pattern::Random {
        add_random_with_symmetry(board, random_seed, random_symmetry_mode);
    } else {
        board.add_pattern_with_seed(pattern, random_seed);
    }
}

fn add_random_with_symmetry(
    board: &mut Board<HEIGHT, WIDTH>,
    random_seed: u32,
    random_symmetry_mode: RandomSymmetryMode,
) {
    let mut random_seed = random_seed;
    let mut next_random_cell = || {
        random_seed = random_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (random_seed & 0x100) != 0
    };

    let mut set_orbit = |positions: &[(usize, usize)]| {
        let random_state = next_random_cell();
        for (row_index, col_index) in positions {
            board.cells[*row_index % HEIGHT][*col_index % WIDTH] = random_state;
        }
    };

    let fill_plain_random =
        |board: &mut Board<HEIGHT, WIDTH>, next_random_cell: &mut dyn FnMut() -> bool| {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    board.cells[row_index][col_index] = next_random_cell();
                }
            }
        };

    let center_col = WIDTH.saturating_sub(1) / 2;
    let center_row = HEIGHT.saturating_sub(1) / 2;
    let col_center_mirror = |col_index: usize| (2 * center_col + WIDTH - col_index) % WIDTH;
    let row_center_mirror = |row_index: usize| (2 * center_row + HEIGHT - row_index) % HEIGHT;

    match random_symmetry_mode {
        RandomSymmetryMode::None => fill_plain_random(board, &mut next_random_cell),
        RandomSymmetryMode::LeftRightNoCenter => {
            let half_width = WIDTH.div_ceil(2);
            for row_index in 0..HEIGHT {
                for col_index in 0..half_width {
                    let mirror_col = (WIDTH + WIDTH.saturating_sub(1) - col_index) % WIDTH;
                    set_orbit(&[(row_index, col_index), (row_index, mirror_col)]);
                }
            }
        }
        RandomSymmetryMode::LeftRightCentered => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    let mirror_col = col_center_mirror(col_index);
                    if col_index <= mirror_col {
                        set_orbit(&[(row_index, col_index), (row_index, mirror_col)]);
                    }
                }
            }
        }
        RandomSymmetryMode::FourWayNoCenter => {
            let half_width = WIDTH.div_ceil(2);
            let half_height = HEIGHT.div_ceil(2);
            for row_index in 0..half_height {
                for col_index in 0..half_width {
                    let mirror_row = (HEIGHT + HEIGHT.saturating_sub(1) - row_index) % HEIGHT;
                    let mirror_col = (WIDTH + WIDTH.saturating_sub(1) - col_index) % WIDTH;
                    set_orbit(&[
                        (row_index, col_index),
                        (row_index, mirror_col),
                        (mirror_row, col_index),
                        (mirror_row, mirror_col),
                    ]);
                }
            }
        }
        RandomSymmetryMode::FourWayCentered => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    let mirror_row = row_center_mirror(row_index);
                    let mirror_col = col_center_mirror(col_index);
                    if row_index <= mirror_row && col_index <= mirror_col {
                        set_orbit(&[
                            (row_index, col_index),
                            (row_index, mirror_col),
                            (mirror_row, col_index),
                            (mirror_row, mirror_col),
                        ]);
                    }
                }
            }
        }
        RandomSymmetryMode::DiagonalNoCenter => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    let mirror_row = HEIGHT.saturating_sub(1).saturating_sub(col_index);
                    let mirror_col = WIDTH.saturating_sub(1).saturating_sub(row_index);
                    if row_index < mirror_row || (row_index == mirror_row && col_index <= mirror_col)
                    {
                        set_orbit(&[(row_index, col_index), (mirror_row, mirror_col)]);
                    }
                }
            }
        }
        RandomSymmetryMode::DiagonalCentered => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    if row_index <= col_index {
                        set_orbit(&[(row_index, col_index), (col_index, row_index)]);
                    }
                }
            }
        }
        RandomSymmetryMode::DiagonalFourWayNoCenter => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    let point = (row_index, col_index);
                    let point_t = (col_index, row_index);
                    let point_a = (
                        HEIGHT.saturating_sub(1).saturating_sub(col_index),
                        WIDTH.saturating_sub(1).saturating_sub(row_index),
                    );
                    let point_at = (point_a.1, point_a.0);
                    if point <= point_t && point <= point_a && point <= point_at {
                        set_orbit(&[point, point_t, point_a, point_at]);
                    }
                }
            }
        }
        RandomSymmetryMode::DiagonalFourWayCentered => {
            for row_index in 0..HEIGHT {
                for col_index in 0..WIDTH {
                    let point = (row_index, col_index);
                    let point_t = (col_index, row_index);
                    let point_c = (row_center_mirror(row_index), col_center_mirror(col_index));
                    let point_ct = (point_c.1, point_c.0);
                    if point <= point_t && point <= point_c && point <= point_ct {
                        set_orbit(&[point, point_t, point_c, point_ct]);
                    }
                }
            }
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
