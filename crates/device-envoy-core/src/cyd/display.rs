//! Display-only data, asset, and drawing plumbing for the CYD's `cyd` device
//! abstraction.
//!
//! The primary type is [`DrawItem`]; see [`CydDisplay::draw_items`](super::CydDisplay::draw_items)
//! for the canonical draw loop that consumes them.

mod contiguous_pixels;
mod draw_item;
mod orientation;
mod tga;
pub mod tiling;

use core::{convert::Infallible, future::Future};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point},
    primitives::Rectangle,
};

use crate::pixel_target::PixelTarget;

pub(crate) use contiguous_pixels::ContiguousPixels;
pub use draw_item::{DrawItem, Image565View};
pub use orientation::Orientation;
pub use tga::{Image565Fixed, Image888Fixed, MaskFixed, mask_byte_count};

pub use crate::__cyd_tga as tga;

/// A borrowed or owned rectangular RGB565 pixel buffer.
///
/// See [`super::CydDisplay::flush_at`] for the primary consumer.
// TODO Consider introducing a concrete `RectanglePixelsView` containing the
// dimensions and pixel slice, then reducing this trait to only `view`. Buffer
// inspection and iteration could live on that common borrowed representation.
pub trait RectanglePixels {
    /// Buffer width in pixels.
    ///
    /// See the [`RectanglePixels` trait documentation](Self) for usage.
    fn width(&self) -> usize;
    /// Buffer height in pixels.
    ///
    /// See the [`RectanglePixels` trait documentation](Self) for usage.
    fn height(&self) -> usize;
    /// Row-major RGB565 pixels.
    ///
    /// See the [`RectanglePixels` trait documentation](Self) for usage.
    fn raw_pixels(&self) -> &[u16];
}

/// A single in-progress frame: a `Rgb565` draw target that can be flushed.
///
/// Also a [`PixelTarget`] so projected linkage draw items can render into it.
/// See the [Cyd trait documentation](super::Cyd) for an end-to-end example that
/// creates, writes, and flushes a frame.
pub trait CydFrame: DrawTarget<Color = Rgb565, Error = Infallible> + PixelTarget {
    /// Error returned when flushing this frame to the panel.
    type Error;

    /// This frame's tile top-left in screen coordinates.
    ///
    /// This point is subtracted from input drawing commands before pixels reach
    /// this frame's local backing buffer. Regular, non-tiled frames use `(0, 0)`.
    #[must_use]
    fn tile_top_left(&self) -> Point {
        Point::zero()
    }

    /// This frame's rectangle (top-left and size) in physical-screen coordinates.
    ///
    /// See the [Cyd trait documentation](super::Cyd) for a usage example.
    fn rectangle(&self) -> Rectangle;

    /// Fill this frame with an explicit color and return `self`.
    ///
    /// See the [Cyd trait documentation](super::Cyd) for a usage example.
    fn fill(&mut self, color: Rgb565) -> &mut Self;

    /// Clear this frame with the display's default background color.
    ///
    /// Unlike [`CydDisplay::clear`](super::CydDisplay::clear), this only
    /// updates the frame buffer and does not immediately write to the panel.
    fn clear(&mut self) -> &mut Self;

    /// Draw `text` at the frame's top-left using the device default font and
    /// foreground color. Returns `&mut Self` for chaining.
    ///
    /// See the [Cyd trait documentation](super::Cyd) for a usage example.
    fn write_text(&mut self, text: &str) -> &mut Self;

    /// Bulk-copy a full-frame, row-major RGB565 buffer into this frame.
    ///
    /// This is the fast path for a full-screen background: a single
    /// `copy_from_slice` instead of the per-pixel [`DrawTarget`] path (on the
    /// esp32 the per-pixel path makes the ballet loop ~1/3 slower). `src` must
    /// hold exactly one entry per frame pixel — i.e. the source image's
    /// dimensions must match the frame's. A mismatch returns
    /// [`crate::Error::CopySize`] rather than panicking or silently corrupting
    /// the buffer.
    ///
    /// See [`Image565Fixed::copy_to`] for the primary convenience wrapper.
    fn copy_from_565(&mut self, src: &[u16]) -> crate::Result<()>;

    /// Present the frame's pixels at its rectangle's top-left (screen coordinates).
    ///
    /// The frame was created over a [`Rectangle`] by [`super::CydDisplay::frame_mut`],
    /// so it already knows where it lives and needs no position argument.
    ///
    /// The returned future is the render loop's frame boundary. On the MCU it
    /// flushes over SPI and resolves immediately; on WASM it awaits the next
    /// browser animation frame, blits to the canvas, then resolves — so a
    /// platform-neutral `loop { draw; flush().await?; }` paces itself to
    /// each device's natural present point without inverting into a state
    /// machine.
    ///
    /// See the [Cyd trait documentation](super::Cyd) for a usage example.
    fn flush(&mut self) -> impl Future<Output = Result<(), <Self as CydFrame>::Error>>;
}
