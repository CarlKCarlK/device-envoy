use device_envoy_core::{led_strip::RGB8, led2d::Frame2d};

/// Maximum number of backtracking iterations before giving up on the predecessor search.
pub const MAX_SEARCH_ITERATIONS: u32 = 500_000;
/// Search only this many cells away from currently-live target cells.
pub const DEFAULT_PREDECESSOR_SEARCH_RADIUS: usize = 1;
/// Number of generations before auto-reset when stasis is detected.
pub const STASIS_RESET_GENERATIONS: u8 = 15;
/// Number of generations before auto-reset when live-cell counts alternate perfectly.
pub const ALTERNATING_STASIS_RESET_GENERATIONS: u8 = 30;

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

/// Random generation symmetry modes for [`Pattern::Random`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomSymmetryMode {
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

impl RandomSymmetryMode {
    /// Return the next random symmetry mode in display-cycle order.
    #[must_use]
    pub const fn next(self) -> Self {
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

    const fn should_use_plain_random<const H: usize, const W: usize>(self) -> bool {
        // Symmetry modes are defined only for even square boards. Rectangles and any
        // odd dimension intentionally fall back to the same random generation as `None`.
        matches!(self, Self::None)
            || W == 0
            || H == 0
            || !W.is_multiple_of(2)
            || !H.is_multiple_of(2)
            || W != H
    }
}

/// Tracks Conway stasis/empty-board conditions used to decide when to auto-reset a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoResetTracker {
    unchanged_live_count_generations: u8,
    alternating_live_count_generations: u8,
    previous_live_count: Option<u16>,
    last_live_count: u16,
    empty_generations: u8,
}

impl AutoResetTracker {
    /// Create tracker state initialized from `board`.
    #[must_use]
    pub fn new<const H: usize, const W: usize>(board: &Board<H, W>) -> Self {
        Self {
            unchanged_live_count_generations: 0,
            alternating_live_count_generations: 0,
            previous_live_count: None,
            last_live_count: board.count_live_cells(),
            empty_generations: 0,
        }
    }

    /// Reset all tracking counters and reinitialize with the current board.
    pub fn reset<const H: usize, const W: usize>(&mut self, board: &Board<H, W>) {
        self.unchanged_live_count_generations = 0;
        self.alternating_live_count_generations = 0;
        self.previous_live_count = None;
        self.last_live_count = board.count_live_cells();
        self.empty_generations = 0;
    }

    /// Observe one generation and return `true` when the current pattern should auto-reset.
    #[must_use]
    pub fn observe_generation<const H: usize, const W: usize>(
        &mut self,
        board: &Board<H, W>,
        pattern: Pattern,
    ) -> bool {
        let live_cell_count = board.count_live_cells();

        if matches!(pattern, Pattern::Random | Pattern::Cross) {
            let last_live_count = self.last_live_count;
            if live_cell_count == self.last_live_count {
                self.unchanged_live_count_generations =
                    self.unchanged_live_count_generations.saturating_add(1);
            } else {
                self.unchanged_live_count_generations = 0;
            }
            if live_cell_count != last_live_count {
                if self.previous_live_count == Some(live_cell_count) {
                    self.alternating_live_count_generations =
                        self.alternating_live_count_generations.saturating_add(1);
                } else {
                    self.alternating_live_count_generations = 1;
                }
            } else {
                self.alternating_live_count_generations = 0;
            }
            self.previous_live_count = Some(last_live_count);
            self.last_live_count = live_cell_count;

            if self.unchanged_live_count_generations >= STASIS_RESET_GENERATIONS
                || self.alternating_live_count_generations >= ALTERNATING_STASIS_RESET_GENERATIONS
            {
                return true;
            }
        } else if live_cell_count == 0 {
            self.empty_generations = self.empty_generations.saturating_add(1);
            if self.empty_generations >= STASIS_RESET_GENERATIONS {
                return true;
            }
        } else {
            self.empty_generations = 0;
            self.alternating_live_count_generations = 0;
            self.previous_live_count = Some(live_cell_count);
            self.last_live_count = live_cell_count;
        }

        false
    }
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

        for (row_index, next_row) in next_cells.iter_mut().enumerate() {
            for (col_index, next_cell) in next_row.iter_mut().enumerate() {
                let live_neighbors = self.count_live_neighbors(row_index, col_index);
                let is_alive = self.cells[row_index][col_index];

                *next_cell = matches!(
                    (is_alive, live_neighbors),
                    (true, 2) | (true, 3) | (false, 3)
                );
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
        self.add_pattern_with_seed_and_random_symmetry(
            pattern,
            random_seed,
            RandomSymmetryMode::None,
        );
    }

    /// Add a preset pattern using `random_seed` and `random_symmetry_mode` for [`Pattern::Random`].
    pub fn add_pattern_with_seed_and_random_symmetry(
        &mut self,
        pattern: Pattern,
        random_seed: u32,
        random_symmetry_mode: RandomSymmetryMode,
    ) {
        match pattern {
            Pattern::Glider => self.add_glider(4, 2),
            Pattern::Blinker => self.add_blinker(5, 4),
            Pattern::Toad => self.add_toad(5, 4),
            Pattern::Beacon => self.add_beacon(4, 4),
            Pattern::Lwss => self.add_lwss(5, 6),
            Pattern::Block => self.add_block(5, 4),
            Pattern::Pentadecathlon => self.add_pentadecathlon(),
            Pattern::Random => self.add_random_with_symmetry(random_seed, random_symmetry_mode),
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

    fn add_random_with_symmetry(
        &mut self,
        random_seed: u32,
        random_symmetry_mode: RandomSymmetryMode,
    ) {
        if random_symmetry_mode.should_use_plain_random::<H, W>() {
            self.add_random(random_seed);
            return;
        }

        let mut random_seed = random_seed;
        let mut next_random_cell = || {
            random_seed = random_seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (random_seed & 0x100) != 0
        };

        let mut set_orbit = |positions: &[(usize, usize)]| {
            let random_state = next_random_cell();
            for (row_index, col_index) in positions {
                self.cells[*row_index % H][*col_index % W] = random_state;
            }
        };

        let center_col = W.saturating_sub(1) / 2;
        let center_row = H.saturating_sub(1) / 2;
        let col_center_mirror = |col_index: usize| (2 * center_col + W - col_index) % W;
        let row_center_mirror = |row_index: usize| (2 * center_row + H - row_index) % H;

        match random_symmetry_mode {
            RandomSymmetryMode::None => unreachable!(),
            RandomSymmetryMode::LeftRightNoCenter => {
                let half_width = W.div_ceil(2);
                for row_index in 0..H {
                    for col_index in 0..half_width {
                        let mirror_col = (W + W.saturating_sub(1) - col_index) % W;
                        set_orbit(&[(row_index, col_index), (row_index, mirror_col)]);
                    }
                }
            }
            RandomSymmetryMode::LeftRightCentered => {
                for row_index in 0..H {
                    for col_index in 0..W {
                        let mirror_col = col_center_mirror(col_index);
                        if col_index <= mirror_col {
                            set_orbit(&[(row_index, col_index), (row_index, mirror_col)]);
                        }
                    }
                }
            }
            RandomSymmetryMode::FourWayNoCenter => {
                let half_width = W.div_ceil(2);
                let half_height = H.div_ceil(2);
                for row_index in 0..half_height {
                    for col_index in 0..half_width {
                        let mirror_row = (H + H.saturating_sub(1) - row_index) % H;
                        let mirror_col = (W + W.saturating_sub(1) - col_index) % W;
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
                for row_index in 0..H {
                    for col_index in 0..W {
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
                for row_index in 0..H {
                    for col_index in 0..W {
                        let mirror_row = H.saturating_sub(1).saturating_sub(col_index);
                        let mirror_col = W.saturating_sub(1).saturating_sub(row_index);
                        if row_index < mirror_row
                            || (row_index == mirror_row && col_index <= mirror_col)
                        {
                            set_orbit(&[(row_index, col_index), (mirror_row, mirror_col)]);
                        }
                    }
                }
            }
            RandomSymmetryMode::DiagonalCentered => {
                for row_index in 0..H {
                    for col_index in 0..W {
                        if row_index <= col_index {
                            set_orbit(&[(row_index, col_index), (col_index, row_index)]);
                        }
                    }
                }
            }
            RandomSymmetryMode::DiagonalFourWayNoCenter => {
                for row_index in 0..H {
                    for col_index in 0..W {
                        let point = (row_index, col_index);
                        let point_t = (col_index, row_index);
                        let point_a = (
                            H.saturating_sub(1).saturating_sub(col_index),
                            W.saturating_sub(1).saturating_sub(row_index),
                        );
                        let point_at = (point_a.1, point_a.0);
                        if point <= point_t && point <= point_a && point <= point_at {
                            set_orbit(&[point, point_t, point_a, point_at]);
                        }
                    }
                }
            }
            RandomSymmetryMode::DiagonalFourWayCentered => {
                for row_index in 0..H {
                    for col_index in 0..W {
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
    use super::{
        ALTERNATING_STASIS_RESET_GENERATIONS, AutoResetTracker, Board, Pattern, RandomSymmetryMode,
        STASIS_RESET_GENERATIONS,
    };

    const SYMMETRY_MODES: &[RandomSymmetryMode] = &[
        RandomSymmetryMode::LeftRightNoCenter,
        RandomSymmetryMode::LeftRightCentered,
        RandomSymmetryMode::FourWayNoCenter,
        RandomSymmetryMode::FourWayCentered,
        RandomSymmetryMode::DiagonalNoCenter,
        RandomSymmetryMode::DiagonalCentered,
        RandomSymmetryMode::DiagonalFourWayNoCenter,
        RandomSymmetryMode::DiagonalFourWayCentered,
    ];

    #[test]
    fn random_symmetry_is_only_enabled_for_even_square_boards() {
        for random_symmetry_mode in SYMMETRY_MODES {
            assert!(!random_symmetry_mode.should_use_plain_random::<16, 16>());
            assert!(random_symmetry_mode.should_use_plain_random::<16, 8>());
            assert!(random_symmetry_mode.should_use_plain_random::<15, 15>());
            assert!(random_symmetry_mode.should_use_plain_random::<16, 15>());
            assert!(random_symmetry_mode.should_use_plain_random::<15, 16>());
        }
    }

    #[test]
    fn random_symmetry_none_always_uses_plain_random() {
        assert!(RandomSymmetryMode::None.should_use_plain_random::<16, 16>());
        assert!(RandomSymmetryMode::None.should_use_plain_random::<16, 8>());
        assert!(RandomSymmetryMode::None.should_use_plain_random::<15, 15>());
    }

    #[test]
    fn auto_reset_triggers_after_stasis_threshold() {
        let mut board = Board::<16, 16>::new();
        board.set_alive(0, 0);

        let mut auto_reset_tracker = AutoResetTracker::new(&board);
        for _ in 0..STASIS_RESET_GENERATIONS - 1 {
            assert!(!auto_reset_tracker.observe_generation(&board, Pattern::Random));
        }
        assert!(auto_reset_tracker.observe_generation(&board, Pattern::Random));
    }

    #[test]
    fn auto_reset_triggers_after_alternating_live_count_threshold() {
        let mut board_one_live = Board::<16, 16>::new();
        board_one_live.set_alive(0, 0);

        let mut board_two_live = Board::<16, 16>::new();
        board_two_live.set_alive(0, 0);
        board_two_live.set_alive(0, 1);

        let mut auto_reset_tracker = AutoResetTracker::new(&board_one_live);

        for generation in 0..ALTERNATING_STASIS_RESET_GENERATIONS {
            let board = if generation % 2 == 0 {
                &board_two_live
            } else {
                &board_one_live
            };
            assert_eq!(
                auto_reset_tracker.observe_generation(board, Pattern::Random),
                generation + 1 == ALTERNATING_STASIS_RESET_GENERATIONS
            );
        }
    }
}
