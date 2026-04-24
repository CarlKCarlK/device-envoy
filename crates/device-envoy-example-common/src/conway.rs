use device_envoy_core::{
    ir::kepler::{IrKepler, KeplerKeys},
    led_strip::RGB8,
    led2d::{Frame2d, Led2d},
};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Instant, Timer};
use smart_leds::colors;

const STASIS_RESET_GENERATIONS: u8 = 15;

/// Maximum number of backtracking iterations before giving up on the predecessor search.
const MAX_SEARCH_ITERATIONS: u32 = 500_000;

/// Color used to visualize the predecessor search in progress.
const SEARCH_COLOR: RGB8 = colors::RED;

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
                    Either::Second(button) => match button {
                        KeplerKeys::Power => {
                            display_power = DisplayPower::Off;
                            led2d.write_frame(Frame2d::<W, H>::new());
                        }
                        KeplerKeys::Num(number) => {
                            if number < PATTERNS.len() as u8 {
                                pattern_index = number as usize;
                                reset_board_for_pattern(
                                    &mut board,
                                    pattern_index,
                                    &mut stasis_tracker,
                                    &mut empty_tracker,
                                );
                            }
                        }
                        KeplerKeys::Minus => {
                            speed_mode = speed_mode.slower();
                        }
                        KeplerKeys::Plus => {
                            speed_mode = speed_mode.faster();
                        }
                        KeplerKeys::Next => {
                            if paused {
                                board.step();
                            }
                        }
                        KeplerKeys::Prev => {
                            if let Some(predecessor) =
                                find_predecessor(&board, &led2d).await
                            {
                                board = predecessor;
                                stasis_tracker = (0, 0);
                                empty_tracker = 0;
                            }
                        }
                        KeplerKeys::PlayPause => {
                            paused = !paused;
                        }
                        KeplerKeys::Mode => {
                            color_index = (color_index + 1) % ALIVE_COLORS.len();
                            alive_color = ALIVE_COLORS[color_index];
                        }
                        _ => {}
                    },
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
/// Assigned alive cells appear in `search_color`; all other cells are black.
fn search_frame<const W: usize, const H: usize>(
    candidate: &Board<H, W>,
    assigned: &[[bool; W]; H],
    search_color: RGB8,
) -> Frame2d<W, H> {
    let mut frame = Frame2d::<W, H>::new();
    for row_index in 0..H {
        for col_index in 0..W {
            if assigned[row_index][col_index] && candidate.cells[row_index][col_index] {
                frame[(col_index, row_index)] = search_color;
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

/// Try to find a predecessor board: a state that evolves into `target` after one step.
///
/// Uses iterative backtracking, assigning predecessor cells one by one in raster order
/// and pruning branches as soon as a Conway-rule constraint is violated.  Displays the
/// partial candidate board in [`SEARCH_COLOR`] while searching.
///
/// Returns `Some(predecessor)` if found, or `None` if no predecessor exists within
/// [`MAX_SEARCH_ITERATIONS`] iterations.
async fn find_predecessor<const W: usize, const H: usize, L>(
    target: &Board<H, W>,
    led2d: &L,
) -> Option<Board<H, W>>
where
    L: Led2d<W, H>,
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
            return Some(candidate);
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
                return None;
            }
            depth -= 1;
            let prev_row = depth / W;
            let prev_col = depth % W;
            assigned[prev_row][prev_col] = false;
        }

        iteration += 1;
        if iteration >= MAX_SEARCH_ITERATIONS {
            return None;
        }

        // Periodically update the display so the search is visible.
        if iteration % 128 == 0 {
            led2d.write_frame(search_frame(&candidate, &assigned, SEARCH_COLOR));
            Timer::after(Duration::from_millis(1)).await;
        }
    }
}
