//! Display-only data, asset, and drawing plumbing for the CYD's `cyd` device
//! abstraction.
//!
//! This module contains the buffered-frame, callback-tiling, and contiguous
//! streaming drawing mechanisms used by [`CydDisplay`](crate::cyd::CydDisplay).
//! `DrawItem` is a convenience for describing shapes and images inside those
//! workflows; see [`CydDisplay::draw_items`](crate::cyd::CydDisplay::draw_items)
//! for its canonical use.
//! Start with the compiled [`CydFrame`] example for ordinary buffered drawing.

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

/// Compile a supported TGA file into an [`Image888Fixed`](tga::Image888Fixed).
///
/// See the [canonical TGA family example](tga).
pub use crate::__cyd_tga as tga;

/// A single in-progress frame: a `Rgb565` draw target that can be flushed.
///
/// Also a [`PixelTarget`] so projected linkage draw items can render into it.
/// See the [Cyd trait documentation](super::Cyd) for an end-to-end example that
/// creates, writes, and flushes a frame.
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::CydFrame;
/// use embedded_graphics::{
///     pixelcolor::Rgb565,
///     prelude::RgbColor,
///     primitives::Rectangle,
/// };
///
/// fn copy_once<F: CydFrame>(frame: &mut F, pixels: &[u16]) -> device_envoy_core::Result<()> {
///     let rectangle = frame.rectangle();
///     let tile_top_left = frame.tile_top_left();
///     let width = frame.width();
///     let height = frame.height();
///     assert_eq!(rectangle.size.width as usize, width);
///     assert_eq!(rectangle.size.height as usize, height);
///     let local_rectangle = Rectangle::new(rectangle.top_left - tile_top_left, rectangle.size);
///     assert_eq!(local_rectangle.size, rectangle.size);
///     frame.fill(Rgb565::BLACK);
///     CydFrame::clear(frame);
///     frame.write_text("CYD");
///     frame.copy_from_565(pixels)
/// }
/// async fn flush_once<F: CydFrame>(frame: &mut F) -> Result<(), <F as CydFrame>::Error> {
///     frame.flush().await
/// }
/// ```
pub trait CydFrame: DrawTarget<Color = Rgb565, Error = Infallible> + PixelTarget {
    /// Error returned when flushing this frame to the panel.
    /// See the compiled canonical
    /// [CydFrame example](trait.CydFrame.html).
    type Error;

    /// This frame's tile top-left in logical display coordinates.
    ///
    /// This point is subtracted from input drawing commands before pixels reach
    /// this frame's local backing buffer. Regular, non-tiled frames use `(0, 0)`.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    #[must_use]
    fn tile_top_left(&self) -> Point {
        Point::zero()
    }

    /// This frame's rectangle (top-left and size) in logical display coordinates.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    fn rectangle(&self) -> Rectangle;

    /// Fill this frame with an explicit color and return `self`.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    fn fill(&mut self, color: Rgb565) -> &mut Self;

    /// Clear this frame with the display's default background color.
    ///
    /// Unlike [`CydDisplay::clear`](crate::cyd::CydDisplay::clear), this only
    /// updates the frame buffer and does not immediately write to the panel.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    fn clear(&mut self) -> &mut Self;

    /// Draw `text` at frame-local `(0, 0)` using the device default font and
    /// foreground color. Returns `&mut Self` for chaining.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    fn write_text(&mut self, text: &str) -> &mut Self;

    /// Bulk-copy a full-frame, row-major RGB565 buffer into this frame.
    ///
    /// This is the fast path for a full-screen background: a single
    /// `copy_from_slice` instead of the per-pixel
    /// [`DrawTarget`](https://docs.rs/embedded-graphics/latest/embedded_graphics/draw_target/trait.DrawTarget.html) path (on the
    /// esp32 the per-pixel path makes the ballet loop ~1/3 slower). `src` must
    /// hold exactly one entry per frame pixel — i.e. the source image's
    /// dimensions must match the frame's. A mismatch returns
    /// [`crate::Error::CopySize`] rather than panicking or silently corrupting
    /// the buffer.
    ///
    /// See the [canonical `CydFrame` example](CydFrame) and
    /// [`Image565Fixed::copy_to`] for the primary convenience wrapper.
    fn copy_from_565(&mut self, src: &[u16]) -> crate::Result<()>;

    /// Present the frame's pixels at its rectangle's top-left (logical display coordinates).
    ///
    /// The frame was created over a
    /// [`Rectangle`](https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/struct.Rectangle.html)
    /// by [`CydDisplay::frame_mut`](crate::cyd::CydDisplay::frame_mut),
    /// so it already knows where it lives and needs no position argument.
    ///
    /// The returned future is the render loop's frame boundary. On the MCU it
    /// flushes over SPI and resolves immediately; on WASM it awaits the next
    /// browser animation frame, blits to the canvas, then resolves — so a
    /// platform-neutral `loop { draw; flush().await?; }` paces itself to
    /// each device's natural present point without inverting into a state
    /// machine.
    ///
    /// See the [canonical `CydFrame` example](CydFrame).
    fn flush(&mut self) -> impl Future<Output = Result<(), <Self as CydFrame>::Error>>;
}
