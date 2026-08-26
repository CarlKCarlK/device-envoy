//! Splits the screen or a rectangle into a grid of tiles so drawing needs only one small buffer.
//!
//! The CYD draws into a single shared pixel buffer that is
//! flushed in pieces. These types describe *where* those pieces live in screen
//! coordinates and *how big* the shared buffer must be, without knowing anything
//! about what an app draws into them.
//!
//! The primary type is [`TileGrid`]: callers give it a rectangular body area
//! and the number of tile columns and rows; it derives the per-tile size with
//! ceiling division and clips the final column/row to the rectangle edges. See
//! [`CydDisplay::tiles`] for the canonical tiled draw loop, and use
//! [`embedded_graphics::primitives::Rectangle`] plus
//! [`max_rectangle_pixel_count`] when sizing a shared buffer around fixed
//! regions.

use embedded_graphics::{
    prelude::{Point, Size},
    primitives::Rectangle,
};

use super::super::CydDisplay;

/// Pixel count for a rectangle.
///
/// See the [canonical `TileGrid` sizing example](TileGrid).
#[must_use]
pub const fn rectangle_pixel_count(rectangle: Rectangle) -> usize {
    (rectangle.size.width * rectangle.size.height) as usize
}

/// Maximum pixel count of two rectangles.
///
/// See the [canonical `TileGrid` sizing example](TileGrid).
#[must_use]
pub const fn max_rectangle_pixel_count(first: Rectangle, second: Rectangle) -> usize {
    if rectangle_pixel_count(first) > rectangle_pixel_count(second) {
        rectangle_pixel_count(first)
    } else {
        rectangle_pixel_count(second)
    }
}

/// A rectangular body area split into a grid of `columns` × `rows` tiles.
///
/// `top_left` and `size` describe the rectangle in screen coordinates; callers
/// specify how many tile columns and rows to split it into, and the per-tile
/// size is derived with ceiling division ([`tile_width`](Self::tile_width) /
/// [`tile_height`](Self::tile_height)). The final column and row are clipped to
/// the rectangle's right and bottom edges.
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::tiling::{
///     TileGrid, max_rectangle_pixel_count, rectangle_pixel_count,
/// };
/// use embedded_graphics::{prelude::{Point, Size}, primitives::Rectangle};
///
/// let rectangle = Rectangle::new(Point::zero(), Size::new(320, 240));
/// let grid = TileGrid::new(rectangle.top_left, rectangle.size, 4, 3);
/// assert_eq!(grid.columns(), 4);
/// assert_eq!(grid.rows(), 3);
/// let _tile_width = grid.tile_width();
/// let _tile_height = grid.tile_height();
/// let buffer_pixels = grid.max_tile_pixel_count();
/// assert_eq!(buffer_pixels, 80 * 80);
/// let _ = rectangle_pixel_count(rectangle);
/// let _ = max_rectangle_pixel_count(rectangle, rectangle);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileGrid {
    pub top_left: Point,
    pub size: Size,
    columns: usize,
    rows: usize,
}

impl TileGrid {
    /// Build a grid splitting `size` into `columns` × `rows` tiles.
    ///
    /// Const-asserts that the counts are positive and do not exceed the rectangle's
    /// pixel dimensions, so an over-fine grid fails to compile. See
    /// [`CydDisplay::tiles`] for the canonical draw loop that consumes the grid.
    #[must_use]
    pub const fn new(top_left: Point, size: Size, columns: usize, rows: usize) -> Self {
        assert!(columns > 0, "columns must be greater than zero");
        assert!(rows > 0, "rows must be greater than zero");
        assert!(
            columns <= size.width as usize,
            "columns must not exceed rectangle width in pixels"
        );
        assert!(
            rows <= size.height as usize,
            "rows must not exceed rectangle height in pixels"
        );
        Self {
            top_left,
            size,
            columns,
            rows,
        }
    }

    /// Number of tile columns the rectangle is split into.
    ///
    /// See the [canonical `TileGrid` sizing example](TileGrid).
    #[must_use]
    pub const fn columns(&self) -> usize {
        self.columns
    }

    /// Number of tile rows the rectangle is split into.
    ///
    /// See the [canonical `TileGrid` sizing example](TileGrid).
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Nominal tile width: the rectangle width divided by the column count, rounded up.
    ///
    /// See the [canonical `TileGrid` sizing example](TileGrid).
    #[must_use]
    pub const fn tile_width(&self) -> usize {
        (self.size.width as usize).div_ceil(self.columns)
    }

    /// Nominal tile height: the rectangle height divided by the row count, rounded up.
    ///
    /// See the [canonical `TileGrid` sizing example](TileGrid).
    #[must_use]
    pub const fn tile_height(&self) -> usize {
        (self.size.height as usize).div_ceil(self.rows)
    }

    /// Largest pixel count any single tile can have.
    ///
    /// The biggest tile is the top-left one, whose dimensions are the derived tile
    /// size clipped to the rectangle (in case the rectangle is smaller than one tile).
    ///
    /// See the [canonical `TileGrid` sizing example](TileGrid).
    #[must_use]
    pub const fn max_tile_pixel_count(&self) -> usize {
        let widest = min_usize(self.tile_width(), self.size.width as usize);
        let tallest = min_usize(self.tile_height(), self.size.height as usize);
        widest * tallest
    }

    /// The tile at `(column, row)` as a [`Rectangle`] in physical-screen
    /// coordinates, or `None` if it lies outside the rectangle.
    ///
    /// The final column/row of a grid may be narrower/shorter than the nominal
    /// tile size when the rectangle does not divide evenly by the tile counts, so
    /// always use the returned rectangle's `size` rather than the grid's derived
    /// tile size when allocating a frame.
    #[must_use]
    pub(crate) fn tile(&self, column: usize, row: usize) -> Option<Rectangle> {
        let tile_width = self.tile_width();
        let tile_height = self.tile_height();
        let column_offset = column * tile_width;
        let row_offset = row * tile_height;

        let region_width = self.size.width as usize;
        let region_height = self.size.height as usize;
        if column_offset >= region_width || row_offset >= region_height {
            return None;
        }

        let width = min_usize(tile_width, region_width - column_offset);
        let height = min_usize(tile_height, region_height - row_offset);
        let size = Size::new(width as u32, height as u32);
        let top_left = Point::new(
            self.top_left.x + column_offset as i32,
            self.top_left.y + row_offset as i32,
        );
        Some(Rectangle::new(top_left, size))
    }
}

const fn min_usize(first: usize, second: usize) -> usize {
    if first < second { first } else { second }
}

/// A lending/streaming iterator over a [`TileGrid`]'s tiles.
///
/// Created by [`CydDisplay::tiles`]. This deliberately does *not* implement
/// [`Iterator`]: each yielded frame borrows the device's
/// single reusable frame buffer, so only one frame can be live at a time.
/// Iterate with a `while let Some(mut frame) = tiles.next()` loop.
/// See the [canonical tiled draw loop](CydDisplay::tiles).
pub struct Tiles<'a, C: CydDisplay> {
    cyd: &'a mut C,
    grid: TileGrid,
    column: usize,
    row: usize,
}

impl<'a, C: CydDisplay> Tiles<'a, C> {
    pub(crate) fn new(cyd: &'a mut C, grid: TileGrid) -> Self {
        Self {
            cyd,
            grid,
            column: 0,
            row: 0,
        }
    }
}

impl<C: CydDisplay> Tiles<'_, C> {
    /// Borrow the next tile-backed frame, cleared to the device background
    /// color, or `None` once every tile has been yielded.
    ///
    /// Tiles are visited in row-major order (each row left-to-right), skipping
    /// any `(column, row)` that falls entirely outside the grid rectangle.
    ///
    /// See the [canonical `CydDisplay::tiles` example](CydDisplay::tiles).
    // This is a lending iterator: each yielded frame borrows the device's single
    // reusable frame buffer, so it cannot implement `Iterator` (whose `next`
    // returns an item that outlives the `&mut self` borrow). The `next` name is
    // the intended call shape, so allow the trait-shape lint here.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<C::Frame<'_>> {
        let (columns, rows) = (self.grid.columns(), self.grid.rows());
        loop {
            if self.row >= rows {
                return None;
            }
            let rectangle = self.grid.tile(self.column, self.row);
            self.column += 1;
            if self.column >= columns {
                self.column = 0;
                self.row += 1;
            }
            if let Some(rectangle) = rectangle {
                let tile_top_left = rectangle.top_left;
                return Some(
                    self.cyd
                        .frame_mut_with_tile_top_left(rectangle, tile_top_left),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Body rectangle used by the dance app: 240×286 starting just below a 34 px
    // text band, split into a 3×3 tile grid (derived tile size 80×96).
    const BODY_GRID: TileGrid = TileGrid::new(Point::new(0, 34), Size::new(240, 286), 3, 3);

    #[test]
    fn exact_fit_columns_and_rows() {
        assert_eq!(BODY_GRID.columns(), 3);
        assert_eq!(BODY_GRID.rows(), 3);
        // 240 / 3 = 80, ceil(286 / 3) = 96.
        assert_eq!(BODY_GRID.tile_width(), 80);
        assert_eq!(BODY_GRID.tile_height(), 96);
    }

    #[test]
    fn final_row_is_clipped() {
        // Origin y = 34, rectangle height 286 → last row (row 2) starts at offset
        // 192 and is clipped from 96 to 94 px high.
        let tile = BODY_GRID.tile(0, 2).expect("tile (0, 2) is in range");
        assert_eq!(tile.top_left, Point::new(0, 34 + 192));
        assert_eq!(tile.size.height, 94);
        assert_eq!(tile.size.width, 80);
    }

    #[test]
    fn exact_division_has_no_clipping() {
        // 240×288 rectangle in a 3×3 grid divides evenly into 80×96 tiles.
        let grid = TileGrid::new(Point::new(0, 0), Size::new(240, 288), 3, 3);
        assert_eq!(grid.tile_width(), 80);
        assert_eq!(grid.tile_height(), 96);
        let tile = grid.tile(2, 2).expect("tile (2, 2) is in range");
        assert_eq!(tile.size, Size::new(80, 96));
    }

    #[test]
    fn final_column_and_row_clipping_for_uneven_dimensions() {
        // 250×290 rectangle in a 4×4 grid: tile size ceil(250/4)=63, ceil(290/4)=73.
        // Last column clips to 250 - 3*63 = 61 px, last row to 290 - 3*73 = 71 px.
        let grid = TileGrid::new(Point::new(5, 7), Size::new(250, 290), 4, 4);
        assert_eq!(grid.columns(), 4);
        assert_eq!(grid.rows(), 4);
        assert_eq!(grid.tile_width(), 63);
        assert_eq!(grid.tile_height(), 73);

        let last_column = grid.tile(3, 0).expect("tile (3, 0) is in range");
        assert_eq!(last_column.top_left, Point::new(5 + 189, 7));
        assert_eq!(last_column.size.width, 61);
        assert_eq!(last_column.size.height, 73);

        let last_row = grid.tile(0, 3).expect("tile (0, 3) is in range");
        assert_eq!(last_row.size.height, 71);

        let corner = grid.tile(3, 3).expect("tile (3, 3) is in range");
        assert_eq!(corner.size, Size::new(61, 71));

        // Out of range in either axis is None.
        assert_eq!(grid.tile(4, 0), None);
        assert_eq!(grid.tile(0, 4), None);
    }

    #[test]
    fn max_tile_pixel_count_is_full_tile() {
        assert_eq!(BODY_GRID.max_tile_pixel_count(), 80 * 96);

        // Rectangle smaller in one axis than its single tile still reports the
        // clipped max: a 1×1 grid over 40×50 has a 40×50 tile.
        let small = TileGrid::new(Point::new(0, 0), Size::new(40, 50), 1, 1);
        assert_eq!(small.max_tile_pixel_count(), 40 * 50);
    }

    #[test]
    #[should_panic(expected = "columns must be greater than zero")]
    fn zero_columns_panics() {
        let _ = TileGrid::new(Point::new(0, 0), Size::new(240, 286), 0, 3);
    }

    #[test]
    #[should_panic(expected = "rows must be greater than zero")]
    fn zero_rows_panics() {
        let _ = TileGrid::new(Point::new(0, 0), Size::new(240, 286), 3, 0);
    }

    #[test]
    #[should_panic(expected = "columns must not exceed rectangle width")]
    fn too_many_columns_panics() {
        let _ = TileGrid::new(Point::new(0, 0), Size::new(4, 286), 5, 3);
    }

    #[test]
    #[should_panic(expected = "rows must not exceed rectangle height")]
    fn too_many_rows_panics() {
        let _ = TileGrid::new(Point::new(0, 0), Size::new(240, 4), 3, 5);
    }

    #[test]
    fn text_band_pixel_count() {
        let text_band = Rectangle::new(Point::new(0, 0), Size::new(240, 34));
        assert_eq!(
            (text_band.size.width * text_band.size.height) as usize,
            8160
        );
    }

    #[test]
    fn tile_grid_is_row_major() {
        // Row-major walk over (column, row): each row left-to-right, top-to-bottom.
        let top_left = |column, row| BODY_GRID.tile(column, row).expect("tile in range").top_left;
        assert_eq!(top_left(0, 0), Point::new(0, 34));
        assert_eq!(top_left(1, 0), Point::new(80, 34));
        assert_eq!(top_left(2, 0), Point::new(160, 34));
        assert_eq!(top_left(0, 1), Point::new(0, 34 + 96));
    }
}
