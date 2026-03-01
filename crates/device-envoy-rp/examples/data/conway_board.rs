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

    /// Load the board state from an array of ASCII row strings.
    /// Each string must be exactly `W` bytes wide, using `#` for alive and `.` for dead.
    fn load_rows(&mut self, rows: [&str; H]) {
        for row_index in 0..H {
            let row_bytes = rows[row_index].as_bytes();
            assert!(row_bytes.len() == W, "row width must match board width");
            for col_index in 0..W {
                self.cells[row_index][col_index] = match row_bytes[col_index] {
                    b'#' => true,
                    b'.' => false,
                    _ => panic!("pattern rows may only contain '.' or '#'"),
                };
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
