#![cfg_attr(not(test), no_std)]

use device_envoy_core::{led_strip::RGB8, led2d::Frame2d};
use smart_leds::colors;

/// Maximum number of backtracking iterations before giving up on the predecessor search.
pub const MAX_SEARCH_ITERATIONS: u32 = 500_000;
/// Search only this many cells away from currently-live target cells.
pub const DEFAULT_PREDECESSOR_SEARCH_RADIUS: usize = 1;
/// How many generations without meaningful change trigger an automatic reset.
pub const STASIS_RESET_GENERATIONS: u8 = 15;
/// How many backtracking iterations to run per cooperative search step.
pub const DEFAULT_SEARCH_ITERATIONS_PER_STEP: u32 = 256;

/// Color used to visualize assigned live cells during predecessor search.
pub const SEARCH_COLOR: RGB8 = colors::RED;
/// Color used to show cells that were assigned as dead during search.
pub const SEARCH_ASSIGNED_DEAD_COLOR: RGB8 = RGB8 { r: 0, g: 0, b: 12 };
/// Color used to hint where the current target generation is alive.
pub const SEARCH_TARGET_HINT_COLOR: RGB8 = RGB8 { r: 0, g: 10, b: 0 };

/// Conway pattern order used by the hardware and web demos.
pub const PATTERNS: [Pattern; 10] = [
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

/// Alive-cell color order used by the hardware and web demos.
pub const ALIVE_COLORS: [RGB8; 6] = [
    colors::LIME,
    colors::CYAN,
    colors::MAGENTA,
    colors::ORANGE,
    colors::YELLOW,
    colors::WHITE,
];

/// Conway pattern presets used by the hardware and web demos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
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

/// Conway's Game of Life board with toroidal wrapping.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Board<const H: usize, const W: usize> {
    pub cells: [[bool; W]; H],
}

impl<const H: usize, const W: usize> Board<H, W> {
    /// Create a new empty board.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cells: [[false; W]; H],
        }
    }

    /// Compute the next generation in place.
    pub fn step(&mut self) {
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

    /// Count the number of live neighbors for a cell.
    #[must_use]
    pub fn count_live_neighbors(&self, row: usize, col: usize) -> u8 {
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

    /// Convert board state to an LED frame with the specified color for alive cells.
    #[must_use]
    pub fn to_frame(&self, alive_color: RGB8) -> Frame2d<W, H> {
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

    /// Add a preset pattern to the board.
    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.add_pattern_with_seed(pattern, 0x9e37_79b9);
    }

    /// Add a preset pattern to the board using `random_seed` for [`Pattern::Random`].
    pub fn add_pattern_with_seed(&mut self, pattern: Pattern, random_seed: u32) {
        match pattern {
            Pattern::Glider => self.add_glider(4, 2),
            Pattern::Blinker => self.add_blinker(5, 4),
            Pattern::Toad => self.add_toad(5, 4),
            Pattern::Beacon => self.add_beacon(4, 4),
            Pattern::Lwss => self.add_lwss(5, 6),
            Pattern::Block => self.add_block(5, 4),
            Pattern::Pentadecathlon => self.add_pentadecathlon(),
            Pattern::Random => self.add_random(random_seed),
            Pattern::Cross => self.add_cross(7, 7),
            Pattern::Custom9 => self.add_custom9(),
        }
    }

    /// Set a cell alive with wrapping.
    pub fn set_alive(&mut self, row_index: usize, col_index: usize) {
        self.cells[row_index % H][col_index % W] = true;
    }

    /// Count live cells.
    #[must_use]
    pub fn count_live_cells(&self) -> u16 {
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

    /// Build a predecessor search mask around currently-live cells.
    #[must_use]
    pub fn predecessor_search_mask(&self, radius: usize) -> [[bool; W]; H] {
        let mut search_mask = [[false; W]; H];
        let radius = radius as isize;

        for row_index in 0..H {
            for col_index in 0..W {
                if self.cells[row_index][col_index] {
                    for row_delta in -radius..=radius {
                        for col_delta in -radius..=radius {
                            let mask_row =
                                ((row_index as isize + row_delta).rem_euclid(H as isize)) as usize;
                            let mask_col =
                                ((col_index as isize + col_delta).rem_euclid(W as isize)) as usize;
                            search_mask[mask_row][mask_col] = true;
                        }
                    }
                }
            }
        }

        search_mask
    }

    /// Check whether this board evolves exactly to `target` after one step.
    #[must_use]
    pub fn evolves_to(&self, target: &Self) -> bool {
        let mut next_board = *self;
        next_board.step();
        next_board.cells == target.cells
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

    fn add_random(&mut self, random_seed: u32) {
        let mut random_seed = random_seed;
        for row_index in 0..H {
            for col_index in 0..W {
                random_seed = random_seed.wrapping_mul(1664525).wrapping_add(1013904223);
                self.cells[row_index][col_index] = (random_seed & 0x100) != 0;
            }
        }
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

impl<const H: usize, const W: usize> Default for Board<H, W> {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of a predecessor search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchOutcome<const H: usize, const W: usize> {
    Found(Board<H, W>),
    NotFound,
    Cancelled,
}

/// Result of advancing a cooperative predecessor search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStep<const H: usize, const W: usize> {
    Progress {
        candidate: Board<H, W>,
        assigned: [[bool; W]; H],
        target: Board<H, W>,
    },
    Outcome(SearchOutcome<H, W>),
}

/// User command understood by the shared Conway app state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConwayCommand {
    Noop,
    Power,
    PlayPause,
    Next,
    Previous,
    Cancel,
    Mode,
    SpeedDown,
    SpeedUp,
    Pattern(usize),
}

/// User-visible Conway status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConwayStatus {
    Ok,
    Paused,
    Searching,
    Found,
    NotFound,
    Cancelled,
    Off,
    Unknown,
}

impl ConwayStatus {
    /// Stable lowercase status text for display adapters.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Paused => "paused",
            Self::Searching => "searching",
            Self::Found => "found",
            Self::NotFound => "not_found",
            Self::Cancelled => "cancelled",
            Self::Off => "off",
            Self::Unknown => "unknown",
        }
    }
}

/// Conway animation speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeedMode {
    Slow,
    Medium,
    Fast,
}

impl SpeedMode {
    /// Move one step slower.
    #[must_use]
    pub const fn slower(self) -> Self {
        match self {
            Self::Slow => Self::Slow,
            Self::Medium => Self::Slow,
            Self::Fast => Self::Medium,
        }
    }

    /// Move one step faster.
    #[must_use]
    pub const fn faster(self) -> Self {
        match self {
            Self::Slow => Self::Medium,
            Self::Medium => Self::Fast,
            Self::Fast => Self::Fast,
        }
    }

    /// Current frame interval in milliseconds.
    #[must_use]
    pub const fn interval_ms(self) -> u32 {
        match self {
            Self::Slow => 500,
            Self::Medium => 160,
            Self::Fast => 50,
        }
    }
}

/// Shared Conway app state.
pub struct Conway<const H: usize, const W: usize> {
    board: Board<H, W>,
    pattern_index: usize,
    search: Option<PredecessorSearch<H, W>>,
    paused: bool,
    display_power_on: bool,
    speed_mode: SpeedMode,
    color_index: usize,
    stasis_tracker: (u8, u16),
    empty_tracker: u8,
    random_seed: u32,
}

impl<const H: usize, const W: usize> Conway<H, W> {
    /// Create a new Conway app state with the default pattern and color.
    #[must_use]
    pub fn new(random_seed: u32) -> Self {
        let pattern_index = 1usize;
        let color_index = 1usize;
        let mut board = Board::new();
        board.add_pattern_with_seed(PATTERNS[pattern_index], random_seed);
        Self {
            board,
            pattern_index,
            search: None,
            paused: false,
            display_power_on: true,
            speed_mode: SpeedMode::Medium,
            color_index,
            stasis_tracker: (0, 0),
            empty_tracker: 0,
            random_seed,
        }
    }

    /// Apply one user command.
    pub fn command(&mut self, command: ConwayCommand) -> ConwayStatus {
        if self.search.is_some() {
            self.search = None;
            if matches!(command, ConwayCommand::Previous | ConwayCommand::Cancel) {
                return ConwayStatus::Cancelled;
            }
        }

        match command {
            ConwayCommand::Noop => ConwayStatus::Ok,
            ConwayCommand::Power => {
                self.display_power_on = !self.display_power_on;
                if self.display_power_on {
                    ConwayStatus::Ok
                } else {
                    ConwayStatus::Off
                }
            }
            ConwayCommand::PlayPause => {
                self.paused = !self.paused;
                if self.paused {
                    ConwayStatus::Paused
                } else {
                    ConwayStatus::Ok
                }
            }
            ConwayCommand::Next => {
                if self.display_power_on && self.paused {
                    self.board.step();
                    self.evaluate_auto_reset();
                }
                ConwayStatus::Ok
            }
            ConwayCommand::Previous => {
                if self.display_power_on {
                    self.search = Some(PredecessorSearch::new(self.board));
                    ConwayStatus::Searching
                } else {
                    ConwayStatus::Off
                }
            }
            ConwayCommand::Cancel => {
                self.search = None;
                ConwayStatus::Cancelled
            }
            ConwayCommand::Mode => {
                self.color_index = (self.color_index + 1) % ALIVE_COLORS.len();
                ConwayStatus::Ok
            }
            ConwayCommand::SpeedDown => {
                self.speed_mode = self.speed_mode.slower();
                ConwayStatus::Ok
            }
            ConwayCommand::SpeedUp => {
                self.speed_mode = self.speed_mode.faster();
                ConwayStatus::Ok
            }
            ConwayCommand::Pattern(pattern_index) => {
                if pattern_index < PATTERNS.len() {
                    self.pattern_index = pattern_index;
                    self.reset_board_for_pattern();
                    ConwayStatus::Ok
                } else {
                    ConwayStatus::Unknown
                }
            }
        }
    }

    /// Advance animation state by one scheduled tick.
    pub fn tick(&mut self) -> ConwayStatus {
        if !self.display_power_on {
            return ConwayStatus::Off;
        }

        if self.search.is_some() {
            return self.advance_search(DEFAULT_SEARCH_ITERATIONS_PER_STEP);
        }

        if self.paused {
            return ConwayStatus::Paused;
        }

        self.board.step();
        self.evaluate_auto_reset();
        ConwayStatus::Ok
    }

    /// Advance a predecessor search by `iteration_budget` iterations.
    pub fn advance_search(&mut self, iteration_budget: u32) -> ConwayStatus {
        let Some(search) = &mut self.search else {
            return ConwayStatus::Ok;
        };

        match search.advance(iteration_budget) {
            SearchStep::Progress { .. } => ConwayStatus::Searching,
            SearchStep::Outcome(SearchOutcome::Found(predecessor)) => {
                self.board = predecessor;
                self.search = None;
                self.stasis_tracker = (0, 0);
                self.empty_tracker = 0;
                ConwayStatus::Found
            }
            SearchStep::Outcome(SearchOutcome::NotFound) => {
                self.search = None;
                ConwayStatus::NotFound
            }
            SearchStep::Outcome(SearchOutcome::Cancelled) => {
                self.search = None;
                ConwayStatus::Cancelled
            }
        }
    }

    /// Current display frame.
    #[must_use]
    pub fn frame(&self) -> Frame2d<W, H> {
        if !self.display_power_on {
            return Frame2d::<W, H>::new();
        }

        if let Some(search) = &self.search {
            let (candidate, assigned, target) = search.progress();
            search_frame(&candidate, &assigned, &target)
        } else {
            self.board.to_frame(ALIVE_COLORS[self.color_index])
        }
    }

    /// Current board state.
    #[must_use]
    pub const fn board(&self) -> Board<H, W> {
        self.board
    }

    /// Current frame interval in milliseconds.
    #[must_use]
    pub const fn tick_interval_ms(&self) -> u32 {
        self.speed_mode.interval_ms()
    }

    /// Whether a predecessor search is active.
    #[must_use]
    pub const fn is_searching(&self) -> bool {
        self.search.is_some()
    }

    fn reset_board_for_pattern(&mut self) {
        self.board = Board::new();
        let random_seed = self.next_random_seed();
        self.board
            .add_pattern_with_seed(PATTERNS[self.pattern_index], random_seed);
        self.stasis_tracker = (0, 0);
        self.empty_tracker = 0;
        self.search = None;
    }

    fn evaluate_auto_reset(&mut self) {
        let live_cell_count = self.board.count_live_cells();
        let current_pattern = PATTERNS[self.pattern_index];

        if matches!(current_pattern, Pattern::Random | Pattern::Cross) {
            let (unchanged_count, last_live_count) = self.stasis_tracker;
            if live_cell_count == last_live_count {
                let new_unchanged_count = unchanged_count + 1;
                self.stasis_tracker = (new_unchanged_count, live_cell_count);

                if new_unchanged_count >= STASIS_RESET_GENERATIONS {
                    let random_seed = self.next_random_seed();
                    self.board
                        .add_pattern_with_seed(current_pattern, random_seed);
                    self.stasis_tracker = (0, 0);
                    self.empty_tracker = 0;
                }
            } else {
                self.stasis_tracker = (1, live_cell_count);
            }
        } else if live_cell_count == 0 {
            self.empty_tracker += 1;
            if self.empty_tracker >= STASIS_RESET_GENERATIONS {
                let random_seed = self.next_random_seed();
                self.board
                    .add_pattern_with_seed(current_pattern, random_seed);
                self.stasis_tracker = (0, 0);
                self.empty_tracker = 0;
            }
        } else {
            self.empty_tracker = 0;
        }
    }

    fn next_random_seed(&mut self) -> u32 {
        self.random_seed = self
            .random_seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.random_seed
    }
}

impl<const H: usize, const W: usize> Default for Conway<H, W> {
    fn default() -> Self {
        Self::new(0x9e37_79b9)
    }
}

/// Cooperative predecessor search state.
pub struct PredecessorSearch<const H: usize, const W: usize> {
    target: Board<H, W>,
    candidate: Board<H, W>,
    search_mask: [[bool; W]; H],
    choices: [[u8; W]; H],
    assigned: [[bool; W]; H],
    depth: usize,
    active_count: usize,
    iteration: u32,
    done: Option<SearchOutcome<H, W>>,
}

impl<const H: usize, const W: usize> PredecessorSearch<H, W> {
    /// Create a new localized predecessor search.
    #[must_use]
    pub fn new(target: Board<H, W>) -> Self {
        Self::new_with_radius(target, DEFAULT_PREDECESSOR_SEARCH_RADIUS)
    }

    /// Create a new predecessor search with a custom live-cell radius.
    #[must_use]
    pub fn new_with_radius(target: Board<H, W>, radius: usize) -> Self {
        let search_mask = target.predecessor_search_mask(radius);
        let mut assigned = [[true; W]; H];
        let active_count = count_search_cells(&search_mask);
        for row_index in 0..H {
            for col_index in 0..W {
                assigned[row_index][col_index] = !search_mask[row_index][col_index];
            }
        }

        Self {
            target,
            candidate: Board::new(),
            search_mask,
            choices: [[0u8; W]; H],
            assigned,
            depth: 0,
            active_count,
            iteration: 0,
            done: None,
        }
    }

    /// Cancel the search.
    pub fn cancel(&mut self) -> SearchStep<H, W> {
        let outcome = SearchOutcome::Cancelled;
        self.done = Some(outcome);
        SearchStep::Outcome(outcome)
    }

    /// Return the current progress snapshot.
    #[must_use]
    pub const fn progress(&self) -> (Board<H, W>, [[bool; W]; H], Board<H, W>) {
        (self.candidate, self.assigned, self.target)
    }

    /// Advance the search by at most `iteration_budget` backtracking iterations.
    pub fn advance(&mut self, iteration_budget: u32) -> SearchStep<H, W> {
        assert!(iteration_budget > 0, "iteration_budget must be positive");
        if let Some(outcome) = self.done {
            return SearchStep::Outcome(outcome);
        }

        let mut budget_remaining = iteration_budget;
        while budget_remaining > 0 {
            budget_remaining -= 1;
            if let Some(outcome) = self.advance_once() {
                self.done = Some(outcome);
                return SearchStep::Outcome(outcome);
            }
        }

        SearchStep::Progress {
            candidate: self.candidate,
            assigned: self.assigned,
            target: self.target,
        }
    }

    fn advance_once(&mut self) -> Option<SearchOutcome<H, W>> {
        if self.depth == self.active_count {
            if self.candidate.evolves_to(&self.target) {
                return Some(SearchOutcome::Found(self.candidate));
            }
            if self.depth == 0 {
                return Some(SearchOutcome::NotFound);
            }
            self.depth -= 1;
            if let Some((prev_row, prev_col)) = search_cell_at(&self.search_mask, self.depth) {
                self.assigned[prev_row][prev_col] = false;
            }
            return None;
        }

        let Some((row, col)) = search_cell_at(&self.search_mask, self.depth) else {
            return Some(SearchOutcome::NotFound);
        };

        let try_value = match self.choices[row][col] {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };

        if let Some(value) = try_value {
            self.choices[row][col] += 1;
            self.candidate.cells[row][col] = value;
            self.assigned[row][col] = true;

            if check_search_constraints(&self.candidate, &self.assigned, &self.target, row, col) {
                self.depth += 1;
            } else {
                self.assigned[row][col] = false;
            }
        } else {
            self.choices[row][col] = 0;
            self.assigned[row][col] = false;
            if self.depth == 0 {
                return Some(SearchOutcome::NotFound);
            }
            self.depth -= 1;
            if let Some((prev_row, prev_col)) = search_cell_at(&self.search_mask, self.depth) {
                self.assigned[prev_row][prev_col] = false;
            }
        }

        self.iteration += 1;
        if self.iteration >= MAX_SEARCH_ITERATIONS {
            return Some(SearchOutcome::NotFound);
        }

        None
    }
}

/// Build a display frame showing current predecessor search progress.
#[must_use]
pub fn search_frame<const W: usize, const H: usize>(
    candidate: &Board<H, W>,
    assigned: &[[bool; W]; H],
    target: &Board<H, W>,
) -> Frame2d<W, H> {
    let mut frame = Frame2d::<W, H>::new();
    for row_index in 0..H {
        for col_index in 0..W {
            if target.cells[row_index][col_index] {
                frame[(col_index, row_index)] = SEARCH_TARGET_HINT_COLOR;
            }

            if assigned[row_index][col_index] {
                frame[(col_index, row_index)] = if candidate.cells[row_index][col_index] {
                    SEARCH_COLOR
                } else {
                    SEARCH_ASSIGNED_DEAD_COLOR
                };
            }
        }
    }
    frame
}

fn check_search_constraints<const W: usize, const H: usize>(
    candidate: &Board<H, W>,
    assigned: &[[bool; W]; H],
    target: &Board<H, W>,
    changed_row: usize,
    changed_col: usize,
) -> bool {
    for row_delta in [-1i32, 0, 1] {
        for col_delta in [-1i32, 0, 1] {
            let check_row = ((changed_row as i32 + row_delta).rem_euclid(H as i32)) as usize;
            let check_col = ((changed_col as i32 + col_delta).rem_euclid(W as i32)) as usize;

            let mut neighborhood_complete = true;
            'check_neighborhood: for neighbor_row_offset in [-1i32, 0, 1] {
                for neighbor_col_offset in [-1i32, 0, 1] {
                    let neighbor_row =
                        ((check_row as i32 + neighbor_row_offset).rem_euclid(H as i32)) as usize;
                    let neighbor_col =
                        ((check_col as i32 + neighbor_col_offset).rem_euclid(W as i32)) as usize;
                    if !assigned[neighbor_row][neighbor_col] {
                        neighborhood_complete = false;
                        break 'check_neighborhood;
                    }
                }
            }

            if neighborhood_complete {
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

fn count_search_cells<const W: usize, const H: usize>(search_mask: &[[bool; W]; H]) -> usize {
    let mut search_cell_count = 0usize;
    for row in search_mask {
        for &is_search_cell in row {
            if is_search_cell {
                search_cell_count += 1;
            }
        }
    }
    search_cell_count
}

fn search_cell_at<const W: usize, const H: usize>(
    search_mask: &[[bool; W]; H],
    target_index: usize,
) -> Option<(usize, usize)> {
    let mut search_cell_index = 0usize;
    for (row_index, row) in search_mask.iter().enumerate() {
        for (col_index, &is_search_cell) in row.iter().enumerate() {
            if is_search_cell {
                if search_cell_index == target_index {
                    return Some((row_index, col_index));
                }
                search_cell_index += 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A horizontal blinker evolves to a vertical blinker and back.
    #[test]
    fn blinker_oscillates() {
        let mut board = Board::<5, 5>::new();
        // Horizontal blinker: row 2, cols 1-3
        board.set_alive(2, 1);
        board.set_alive(2, 2);
        board.set_alive(2, 3);

        board.step();

        // After one step: vertical blinker at rows 1-3, col 2
        let mut expected_vertical = Board::<5, 5>::new();
        expected_vertical.set_alive(1, 2);
        expected_vertical.set_alive(2, 2);
        expected_vertical.set_alive(3, 2);
        assert_eq!(board, expected_vertical);

        board.step();

        // After two steps: back to horizontal blinker
        let mut expected_horizontal = Board::<5, 5>::new();
        expected_horizontal.set_alive(2, 1);
        expected_horizontal.set_alive(2, 2);
        expected_horizontal.set_alive(2, 3);
        assert_eq!(board, expected_horizontal);
    }

    /// Conway::board() returns the current board state.
    #[test]
    fn conway_board_accessor_returns_live_cells() {
        let conway = Conway::<8, 8>::new(0x9e37_79b9);
        let board = conway.board();
        assert!(
            board.count_live_cells() > 0,
            "default random pattern should have live cells"
        );
    }
}
