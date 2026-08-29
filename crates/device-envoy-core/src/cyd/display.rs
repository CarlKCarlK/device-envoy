//! Display-only data, asset, and drawing plumbing for the CYD's `cyd` device
//! abstraction.
//!
//! This module contains the buffered-frame, tiled drawing, and contiguous
//! streaming drawing mechanisms used by [`CydDisplay`](crate::cyd::CydDisplay).
//! `DrawItem` is a convenience for describing shapes and images inside those
//! workflows; see [`CydDisplay::draw_items`](crate::cyd::CydDisplay::draw_items)
//! for an example.
//! Start with the compiled [`CydFrame`] example for ordinary buffered drawing.

mod contiguous_pixels;
mod draw_item;
mod orientation;
mod tga;
pub mod tiling;

use core::{convert::Infallible, future::Future};
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget, primitives::Rectangle};

use crate::pixel_target::PixelTarget;

pub(crate) use contiguous_pixels::ContiguousPixels;
pub use draw_item::{DrawItem, Image565View};
pub use orientation::Orientation;
pub use tga::{Image565Fixed, Image888Fixed, MaskFixed, MaskedDrawable, mask_byte_count};

/// Embeds and decodes a supported TGA file into an [`Image888Fixed`] at compile
/// time.
///
/// The file must be an uncompressed 24-bit BGR or 32-bit BGRA true-color TGA.
/// Alpha is discarded. The output type supplies the expected image dimensions;
/// compilation fails if they do not match the file. The decoded
/// [`Image888Fixed`] can also produce the color-key transparency mask shown in
/// the [`MaskFixed` example](MaskFixed).
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::{Image888Fixed, tga};
///
/// const IMAGE: Image888Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
///     env!("CARGO_MANIFEST_DIR"),
///     "/docs/assets/cyd_fill_contiguous.tga"
/// ));
///
/// assert_eq!(IMAGE.pixels.len(), 45 * 73);
/// ```
pub use crate::__cyd_tga as tga;

/// A single in-progress frame: a `Rgb565` draw target that can be flushed.
///
/// Also a [`PixelTarget`] so projected linkage draw items can render into it.
/// All drawing coordinates are logical display coordinates. A frame only
/// restricts the drawable region and the pixels buffered for presentation.
/// See the [`CydDisplay::frame_mut`](crate::cyd::CydDisplay::frame_mut) example
/// for constructing a frame from a display.
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "cyd_frame_preview",
        "docs/assets/cyd_frame_preview.png"
    )
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::cyd::{CydDisplay, display::CydFrame};
use embedded_graphics::{
    pixelcolor::{Rgb565, RgbColor},
    prelude::{Point, Size},
    primitives::Rectangle,
};

async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    let mut frame = display.frame_mut(Rectangle::new(Point::new(10, 10), Size::new(100, 40)));
    frame.fill(Rgb565::BLUE).write_text("CYD").flush().await
}

# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{mono_font::ascii::FONT_9X15_BOLD, pixelcolor::Rgb888};
# let mut cyd_memory = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# let mut display = cyd_memory.display();
# futures_executor::block_on(draw(&mut display))?;
# let golden_result = assert_framebuffer_matches_expected_png(
#     &cyd_memory,
#     env!("CARGO_MANIFEST_DIR"),
#     "cyd_frame_preview.png",
# );
# assert!(golden_result.is_ok(), "{golden_result:?}");
# Ok::<(), device_envoy_core::memory::Error>(())
```

"#
)]
#[cfg_attr(
    all(feature = "host", feature = "doc-images"),
    doc = "\n![A blue frame containing white CYD text on an in-memory display.][cyd_frame_preview]\n"
)]
pub trait CydFrame: DrawTarget<Color = Rgb565, Error = Infallible> + PixelTarget {
    /// Error returned when presenting the frame.
    type Error;

    /// This frame's rectangle (top-left and size) in logical display coordinates.
    ///
    /// Embedded-graphics'
    /// [`Dimensions::bounding_box`](https://docs.rs/embedded-graphics/latest/embedded_graphics/geometry/trait.Dimensions.html#tymethod.bounding_box)
    /// returns this same rectangle.
    fn rectangle(&self) -> Rectangle;

    /// Fill this frame with an explicit color and return `self`.
    fn fill(&mut self, color: Rgb565) -> &mut Self;

    /// Clear this frame with the display's default background color.
    ///
    /// Unlike [`CydDisplay::clear`](crate::cyd::CydDisplay::clear), this only
    /// updates the frame buffer and does not immediately write to the panel.
    ///
    /// See the [`CydDisplay::frame_mut`](crate::cyd::CydDisplay::frame_mut)
    /// example.
    fn clear(&mut self) -> &mut Self;

    /// Draw `text` at the frame rectangle's top-left using the device default
    /// font and foreground color. Returns `&mut Self` for chaining.
    fn write_text(&mut self, text: &str) -> &mut Self;

    /// Bulk-copy a complete frame-sized, row-major RGB565 image into this frame.
    ///
    /// This copies the image as one contiguous buffer instead of using the
    /// per-pixel
    /// [`DrawTarget`](https://docs.rs/embedded-graphics/latest/embedded_graphics/draw_target/trait.DrawTarget.html)
    /// path. `src` must hold exactly one entry per frame pixel, so the source
    /// image's dimensions must match the frame's. A mismatch returns
    /// [`crate::Error::CopySize`] rather than panicking or silently corrupting
    /// the buffer.
    ///
    /// See the [`Image565Fixed::copy_to`] example for the primary convenience
    /// wrapper.
    fn copy_from_565(&mut self, src: &[u16]) -> crate::Result<()>;

    /// Present the frame's pixels in logical display coordinates.
    ///
    /// The frame was created over a
    /// [`Rectangle`](https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/struct.Rectangle.html)
    /// by [`CydDisplay::frame_mut`](crate::cyd::CydDisplay::frame_mut),
    /// so it already knows where it lives and needs no position argument.
    ///
    /// The returned future is the render loop's frame boundary. Physical
    /// display implementations present the frame directly. The browser
    /// implementation presents it and then awaits the next animation frame.
    /// Therefore, a platform-neutral `loop { draw; flush().await?; }` paces
    /// itself to each display's natural presentation point.
    fn flush(&mut self) -> impl Future<Output = Result<(), <Self as CydFrame>::Error>>;
}
