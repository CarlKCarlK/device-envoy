#![no_std]

use device_envoy_core::{led_strip::RGB8, led2d::Frame2d};

/// Maximum number of backtracking iterations before giving up on the predecessor search.
pub const MAX_SEARCH_ITERATIONS: u32 = 500_000;
/// Search only this many cells away from currently-live target cells.
pub const DEFAULT_PREDECESSOR_SEARCH_RADIUS: usize = 1;

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
