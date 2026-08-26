//! A device abstraction for the "Cheap Yellow Display" (CYD) display and touch parts.
//!
//! Tested on so far:
//!
//! - an integrated ESP32 Cheap Yellow Display board
//! - the same standalone CYD 320×240 SPI display/touch board, driven
//!   externally by both ESP32 and Raspberry Pi Pico 2 setups
//!
//! See [`Cyd`] for the primary trait and usage example. The four drawing
//! strategies are compared together on [`CydDisplay`].

pub mod backend;
pub mod display;
pub mod touch;

use display::{ContiguousPixels, CydFrame};

/// Native panel width in pixels (landscape): 320. The CYD panel is fixed hardware.
pub(crate) const SCREEN_WIDTH: usize = 320;
/// Native panel height in pixels (landscape): 240. The CYD panel is fixed hardware.
pub(crate) const SCREEN_HEIGHT: usize = 240;
/// Total panel pixel count (`SCREEN_WIDTH * SCREEN_HEIGHT` = 76,800).
///
/// ```rust,no_run
/// use device_envoy_core::cyd::SCREEN_PIXELS;
/// // Platform static storage uses this exact size for a full-screen buffer.
/// const PIXEL_BUFFER_SIZE: usize = SCREEN_PIXELS;
/// assert_eq!(PIXEL_BUFFER_SIZE, 320 * 240);
/// ```
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

use crate::pixel_target::rgb565_from_rgb888;
use embedded_graphics::{
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Point, Size},
    primitives::Rectangle,
};

use display::Orientation;
use touch::TouchEvent;

/// A device abstraction for the "Cheap Yellow Display" (CYD) display and touch parts.
///
/// `Cyd` is the core trait for ready-to-use device bundles. It provides borrowed access
/// to calibrated display and touch halves. Generic app code should use `Cyd` to remain
/// compatible with hardware designs where display and touch share underlying resources.
///
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("cyd_trait_preview", "docs/assets/cyd_trait_preview.png")
)]
#[doc = r#"
Generic applications can read calibrated touch events and flush a frame without
depending on a platform crate's concrete component types:

```rust,no_run
use device_envoy_core::cyd::{Cyd, CydDisplay, CydTouch, display::CydFrame};

async fn draw_once<S: Cyd>(device: &mut S) -> Result<(), S::Error> {
    let (display, touch) = device.parts();
    let _touch_event = touch.read()?;
    let mut frame = display.full_frame_mut();
    frame.clear().flush().await?;
    drop(frame);
    let _display = device.display();
    let _orientation = device.orientation();
    Ok(())
}
```
"#]
#[cfg_attr(
    feature = "host",
    doc = r#"

Implementations include the in-memory mock [`CydMemory`](crate::memory::CydMemory), the browser-simulated `CydWasm`, and platform crates for ESP32 and Pico boards.

```rust
use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{CydFrame, DrawItem},
    touch::TouchEvent,
};
use embedded_graphics::pixelcolor::Rgb888;
# use device_envoy_core::memory::CydMemory;
# use device_envoy_core::memory::assert_framebuffer_matches_expected_png;
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     pixelcolor::Rgb565,
#     prelude::{Point, RgbColor, Size},
# };
# futures_executor::block_on(async {
# let mut cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# cyd.push_touch_event(TouchEvent::Down { point: Point::new(160, 120) });
// Create a pixel-buffer covering the whole screen that starts filled with background color.
let (display, touch) = cyd.parts();
let touch_event = touch.read()?;
let mut frame = display.full_frame_mut();

frame.write_text("Hello CYD");
// An app would usually run this in a loop: read touch, draw, flush, repeat.
if let Some(TouchEvent::Down { point } | TouchEvent::Move { point }) = touch_event {
    DrawItem::Circle {
        center: (point.x as f32, point.y as f32),
        pixel_radius: 24.0,
        color: Rgb888::RED,
    }
    .draw(&mut frame);
}
frame.flush().await?;
# assert_eq!(cyd.pixel(160, 120), Rgb565::RED);
# if let Err(error) = assert_framebuffer_matches_expected_png(
#     &cyd,
#     env!("CARGO_MANIFEST_DIR"),
#     "cyd_trait_preview.png",
# ) {
#     panic!("{error}");
# }
# Ok::<(), device_envoy_core::memory::Error>(())
# })?;
# Ok::<(), device_envoy_core::memory::Error>(())
```

![CYD trait preview][cyd_trait_preview]
"#
)]
pub trait Cyd: Sized {
    /// Error returned by both the display and calibrated touch parts.
    type Error;

    type Display: CydDisplay<Error = Self::Error>;
    type Touch: CydTouch<Error = Self::Error>;

    /// Borrow both calibrated halves at once.
    ///
    /// See the [canonical `Cyd` device-loop example](Cyd).
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    /// Borrow the calibrated display half.
    ///
    /// See the [canonical `Cyd` device-loop example](Cyd).
    fn display(&mut self) -> &mut Self::Display {
        self.parts().0
    }

    /// Return the logical orientation of this complete device.
    ///
    /// See the [canonical `Cyd` device-loop example](Cyd).
    fn orientation(&self) -> Orientation;
}

/// A CYD touch source for calibrated, oriented events that apps read.
///
/// [`CydTouch::read`] returns a [`touch::TouchEvent`] carrying an x-y point in
/// the same screen coordinates as the display, or `None` when there is no
/// touch. See the [canonical calibrated-read example](CydTouch::read);
/// applications should not need the platform-author-only backend module.
pub trait CydTouch: Sized {
    /// Error returned when reading touch fails.
    type Error;

    /// Read the next calibrated touch event, if any.
    ///
    /// Returned points are calibrated and oriented into the same logical
    /// coordinates as the display's [`CydDisplay::screen_size`]. Returns
    /// `Ok(None)` when there is no pending touch.
    /// Errors only on a hardware/read failure.
    ///
    /// The canonical calibrated-read example below consumes the already
    /// oriented point directly; applications must not map it a second time.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{CydTouch, touch::TouchEvent};
    ///
    /// fn read_calibrated<T: CydTouch>(touch: &mut T) -> Result<(), T::Error> {
    ///     if let Some(event) = touch.read()? {
    ///         let point = match event {
    ///             TouchEvent::Down { point } | TouchEvent::Move { point } => {
    ///                 // `point` is already in the display's logical coordinates.
    ///                 point
    ///             }
    ///             TouchEvent::Up => return Ok(()),
    ///         };
    ///         consume_point(point);
    ///     }
    ///     Ok(())
    /// }
    ///
    /// fn consume_point(point: embedded_graphics::prelude::Point) {
    ///     // Hit testing and drawing use this logical display coordinate directly.
    ///     assert!(point.x >= 0 && point.y >= 0);
    /// }
    /// ```
    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error>;
}

/// A CYD display.
///
/// The screen is a fixed 320×240 RGB565 panel. `CydDisplay` offers three
/// ways to draw, trading memory for flexibility: [`display::CydFrame`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/display/trait.CydFrame.html)s that can be
/// drawn into and flushed to any rectangle on screen; callback-tiled frames (see
/// [`CydDisplay::for_each_tile`]) that cover the screen (or a rectangle) in smaller pieces when
/// memory is tight; and contiguous-pixel methods (see
/// [`CydDisplay::fill_contiguous`]) that stream pixels straight to the screen
/// with virtually no buffering.
///
/// The drawing strategies have deliberately different coordinate and replay semantics:
/// full-screen frames render a complete scene in screen coordinates; regional frames update
/// independent rectangles and use coordinates local to each rectangle; tiled callbacks replay a
/// complete scene once per tile while accepting screen coordinates; and streaming generates the
/// complete row-major raster directly. The host comparison test exercises the same boundary-
/// crossing scene through all four paths.
///
pub trait CydDisplay: backend::DisplayBackend {
    /// Oriented screen size for the configured orientation.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    ///
    /// fn inspect<D: CydDisplay>(display: &D) {
    ///     let _size = display.screen_size();
    ///     let _background = display.background_color();
    ///     let foreground = display.foreground_color();
    ///     let _background565 = display.background_565();
    ///     let _foreground565 = display.foreground_565();
    ///     let _native = display.to_rgb565(foreground);
    /// }
    /// ```
    ///
    #[cfg_attr(
        feature = "host",
        doc = r#"

```rust
use device_envoy_core::cyd::CydDisplay;
use device_envoy_core::memory::CydMemory;
use embedded_graphics::pixelcolor::Rgb888;
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     prelude::{RgbColor, Size},
# };
let display = CydMemory::new(
    Size::new(320, 240),
    Rgb888::BLACK,
    Rgb888::WHITE,
    &FONT_9X15_BOLD,
)
.display();
assert_eq!(display.screen_size(), Size::new(320, 240));
assert_eq!(display.background_565(), display.to_rgb565(display.background_color()));
assert_eq!(display.foreground_565(), display.to_rgb565(display.foreground_color()));
# Ok::<(), device_envoy_core::memory::Error>(())
```
"#
    )]
    fn screen_size(&self) -> Size;

    /// The device default background color.
    ///
    /// See the [`CydDisplay::screen_size`] example covering the device getter family.
    fn background_color(&self) -> Rgb888;

    /// The device default foreground/text color.
    ///
    /// See the [`CydDisplay::screen_size`] example covering the device getter family.
    fn foreground_color(&self) -> Rgb888;

    /// The device default background color in the native `Rgb565` format.
    ///
    /// See the [`CydDisplay::screen_size`] example covering the device getter family.
    fn background_565(&self) -> Rgb565;

    /// The device default foreground/text color in the native `Rgb565` format.
    ///
    /// See the [`CydDisplay::screen_size`] example covering the device getter family.
    fn foreground_565(&self) -> Rgb565;

    /// Convert an `Rgb888` color to the device's native `Rgb565` format.
    ///
    /// See the [`CydDisplay::screen_size`] example covering the device getter family.
    fn to_rgb565(&self, color: Rgb888) -> Rgb565 {
        rgb565_from_rgb888(color)
    }

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// Drawing commands use coordinates local to the frame's rectangle. Use
    /// [`CydDisplay::for_each_tile`] when replaying a complete screen-coordinate
    /// scene into smaller buffers.
    ///
    /// See the [canonical frame example](CydDisplay::frame_mut).
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{CydDisplay, display::CydFrame};
    /// use embedded_graphics::{prelude::Point, primitives::Rectangle};
    ///
    /// async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let mut frame = display.frame_mut(Rectangle::new(Point::zero(), display.screen_size()));
    ///     frame.clear().flush().await
    /// }
    /// ```
    ///
    #[cfg_attr(
        feature = "doc-images",
        doc = ::embed_doc_image::embed_image!(
            "cyd_frame_mut_preview",
            "docs/assets/cyd_frame_mut_preview.png"
        )
    )]
    #[cfg_attr(
        feature = "host",
        doc = r#"

```rust
use device_envoy_core::cyd::{CydDisplay, display::CydFrame};
use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
use embedded_graphics::{
    pixelcolor::{Rgb565, Rgb888},
    prelude::{RgbColor, Size},
    primitives::Rectangle,
};
# use embedded_graphics::{mono_font::ascii::FONT_9X15_BOLD, prelude::Point};
# futures_executor::block_on(async {
# let memory_cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# let mut display = memory_cyd.display();
let mut frame = display.frame_mut(Rectangle::new(Point::new(10, 10), Size::new(50, 40)));
frame.fill(Rgb565::RED);
frame.flush().await?;
# if let Err(error) = assert_framebuffer_matches_expected_png(
#     &memory_cyd,
#     env!("CARGO_MANIFEST_DIR"),
#     "cyd_frame_mut_preview.png",
# ) {
#     panic!("{error}");
# }
# Ok::<(), device_envoy_core::memory::Error>(())
# })?;
# Ok::<(), device_envoy_core::memory::Error>(())
```

![CYD frame preview][cyd_frame_mut_preview]
"#
    )]
    fn frame_mut(&mut self, rectangle: Rectangle) -> Self::Frame<'_> {
        backend::DisplayBackend::frame_mut_with_tile_top_left(self, rectangle, Point::zero())
    }

    /// Borrow a full-screen frame, cleared to the device background color.
    ///
    /// See the [canonical `Cyd` device-loop example](Cyd).
    fn full_frame_mut(&mut self) -> Self::Frame<'_> {
        self.frame_mut(Rectangle::new(Point::zero(), self.screen_size()))
    }

    /// Fill `rectangle` immediately in physical-screen coordinates.
    ///
    /// Unlike [`display::CydFrame::fill`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/display/trait.CydFrame.html#tymethod.fill), this is a device-level operation rather than a
    /// frame-local buffered draw. Implementations clip to the physical screen and
    /// treat an empty intersection as a no-op.
    ///
    /// This is the canonical example for the immediate and contiguous operations:
    /// [`CydDisplay::fill_contiguous`], [`CydDisplay::draw_items`], [`CydDisplay::clear`],
    /// and [`CydDisplay::fill`].
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    /// use embedded_graphics::{pixelcolor::Rgb565, prelude::{Point, RgbColor, Size}, primitives::Rectangle};
    ///
    /// async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let rectangle = Rectangle::new(Point::zero(), Size::new(2, 2));
    ///     display.fill_rectangle(rectangle, Rgb565::BLACK)?;
    ///     display.fill_contiguous(rectangle, [Rgb565::RED; 4])?;
    ///     display.draw_items::<1>(rectangle, Rgb565::BLACK, [])?;
    ///     display.clear()?;
    ///     display.fill(Rgb565::WHITE)
    /// }
    /// ```
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Self::Error>;

    /// Fill `rectangle` immediately from row-major native-color pixels.
    ///
    /// Empty rectangles are a no-op.
    ///
    /// See the [canonical streaming example](CydDisplay::fill_contiguous_full).
    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>;

    /// Fill the complete screen immediately from row-major native-color pixels.
    ///
    /// This is the whole-screen counterpart to [`CydDisplay::fill_contiguous`].
    /// It expresses full-screen streaming intent without repeating the complete
    /// screen rectangle. Streaming is an advanced raster path: the caller
    /// generates every pixel in row-major order rather than drawing a scene.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    /// use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
    ///
    /// fn stream<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let pixels = (0..320 * 240).map(|pixel_index| {
    ///         if pixel_index % 320 == 0 { Rgb565::WHITE } else { Rgb565::BLACK }
    ///     });
    ///     display.fill_contiguous_full(pixels)
    /// }
    /// ```
    fn fill_contiguous_full<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        self.fill_contiguous(Rectangle::new(Point::zero(), self.screen_size()), pixels)
    }

    /// Draw projected draw items immediately inside `bounds`.
    ///
    /// See the [canonical immediate-operations example](CydDisplay::fill_rectangle) and
    /// the [`display::DrawItem`](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/display/enum.DrawItem.html) documentation for the draw-item types this consumes.
    fn draw_items<const PIXEL_SOURCE_COUNT: usize>(
        &mut self,
        bounds: Rectangle,
        background_color: Rgb565,
        items: impl IntoIterator<Item = display::DrawItem>,
    ) -> Result<(), Self::Error> {
        let bounds = bounds.intersection(&Rectangle::new(Point::zero(), self.screen_size()));
        let pixel_sources = ContiguousPixels::<PIXEL_SOURCE_COUNT>::from_draw_items(
            bounds,
            background_color,
            items,
        );
        self.fill_contiguous(pixel_sources.bounds(), pixel_sources.iter())
    }

    /// Clear the whole screen to the device default background color.
    ///
    /// New frames already start cleared to this color. This is for immediately
    /// returning the physical screen to the default background between frame
    /// workflows.
    ///
    /// See the [canonical immediate-operations example](CydDisplay::fill_rectangle).
    fn clear(&mut self) -> Result<(), Self::Error> {
        self.fill(self.background_565())
    }

    /// Fill the whole screen with an explicit color.
    ///
    /// See the [canonical immediate-operations example](CydDisplay::fill_rectangle).
    fn fill(&mut self, color: Rgb565) -> Result<(), Self::Error> {
        self.fill_rectangle(Rectangle::new(Point::zero(), self.screen_size()), color)
    }

    /// Draw and flush each tile in `grid` through a synchronous callback.
    ///
    /// The callback receives one screen-coordinate frame at a time; this helper owns the
    /// reusable frame and flush sequence, so callers do not need to handle lending iterator
    /// lifetimes. This is the primary low-memory drawing workflow.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::{cyd::{CydDisplay, display::CydFrame, display::tiling::TileGrid}, UnwrapInfallible};
    /// use embedded_graphics::{Drawable, pixelcolor::Rgb565, prelude::{Point, Primitive, RgbColor, Size}, primitives::{PrimitiveStyle, Rectangle}};
    ///
    /// async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let grid = TileGrid::new(Point::zero(), Size::new(320, 240), 4, 3);
    ///     display.for_each_tile(grid, |frame| {
    ///         frame.fill(Rgb565::BLUE);
    ///         Rectangle::new(Point::new(12, 18), Size::new(20, 12))
    ///             .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
    ///             .draw(frame)
    ///             .unwrap_infallible();
    ///     }).await
    /// }
    /// ```
    fn for_each_tile<'a, F>(
        &'a mut self,
        grid: display::tiling::TileGrid,
        mut draw: F,
    ) -> impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: Sized,
        F: for<'frame> FnMut(&mut Self::Frame<'frame>) + 'a,
    {
        async move {
            let mut tiles = display::tiling::Tiles::new(self, grid);
            while let Some(mut frame) = tiles.next() {
                draw(&mut frame);
                frame.flush().await?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cyd::display::CydFrame;
    use crate::pixel_target::PixelTarget;
    use core::convert::Infallible;
    use embedded_graphics::pixelcolor::WebColors;
    use embedded_graphics::{
        Pixel,
        prelude::{DrawTarget, OriginDimensions},
    };

    // TODO The shared `linkage-blaze-cyd-memory` fake cannot replace this unit-test
    // double directly because a cyd-core <-> cyd-memory dev-dependency cycle gives
    // cyd-core's unit tests a second trait instance, so `CydMemory` no longer
    // implements *this* module's `CydDisplay` trait. Keep this tiny local
    // test double until the trait crate/test layout is refactored to break that cycle.
    struct TestCyd;

    struct TestFrame {
        rectangle: Rectangle,
        tile_top_left: Point,
    }

    impl backend::DisplayBackend for TestCyd {
        type Error = Infallible;
        type Frame<'a> = TestFrame;

        fn frame_mut_with_tile_top_left(
            &mut self,
            rectangle: Rectangle,
            tile_top_left: Point,
        ) -> TestFrame {
            TestFrame {
                rectangle,
                tile_top_left,
            }
        }
    }

    impl CydDisplay for TestCyd {
        fn screen_size(&self) -> Size {
            Size::new(320, 240)
        }

        fn background_color(&self) -> Rgb888 {
            Rgb888::CSS_BLACK
        }

        fn foreground_color(&self) -> Rgb888 {
            Rgb888::CSS_WHITE
        }

        fn background_565(&self) -> Rgb565 {
            self.to_rgb565(self.background_color())
        }

        fn foreground_565(&self) -> Rgb565 {
            self.to_rgb565(self.foreground_color())
        }

        fn fill_rectangle(
            &mut self,
            _rectangle: Rectangle,
            _color: Rgb565,
        ) -> Result<(), Infallible> {
            Ok(())
        }

        fn fill_contiguous<I>(
            &mut self,
            _rectangle: Rectangle,
            _pixels: I,
        ) -> Result<(), Infallible>
        where
            I: IntoIterator<Item = Rgb565>,
        {
            Ok(())
        }
    }

    impl DrawTarget for TestFrame {
        type Color = Rgb565;
        type Error = Infallible;

        fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            Ok(())
        }
    }

    impl OriginDimensions for TestFrame {
        fn size(&self) -> Size {
            self.rectangle.size
        }
    }

    impl PixelTarget for TestFrame {
        fn width(&self) -> usize {
            self.rectangle.size.width as usize
        }

        fn height(&self) -> usize {
            self.rectangle.size.height as usize
        }

        fn put_pixel(&mut self, _x: usize, _y: usize, _color: Rgb888) {}
    }

    impl CydFrame for TestFrame {
        type Error = Infallible;

        fn tile_top_left(&self) -> Point {
            self.tile_top_left
        }

        fn rectangle(&self) -> Rectangle {
            self.rectangle
        }

        fn fill(&mut self, _color: Rgb565) -> &mut Self {
            self
        }

        fn clear(&mut self) -> &mut Self {
            self
        }

        fn write_text(&mut self, _text: &str) -> &mut Self {
            self
        }

        fn copy_from_565(&mut self, _src: &[u16]) -> crate::Result<()> {
            Ok(())
        }

        async fn flush(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    #[test]
    fn tiled_frames_use_screen_tile_top_left() {
        let mut cyd = TestCyd;
        let grid = display::tiling::TileGrid::new(Point::new(10, 20), Size::new(8, 6), 2, 2);
        let mut tiles = display::tiling::Tiles::new(&mut cyd, grid);

        {
            let first = tiles.next().expect("first tile exists");
            assert_eq!(
                first.rectangle(),
                Rectangle::new(Point::new(10, 20), Size::new(4, 3))
            );
            assert_eq!(first.tile_top_left(), Point::new(10, 20));
        }

        {
            let second = tiles.next().expect("second tile exists");
            assert_eq!(
                second.rectangle(),
                Rectangle::new(Point::new(14, 20), Size::new(4, 3))
            );
            assert_eq!(second.tile_top_left(), Point::new(14, 20));
        }

        let third = tiles.next().expect("third tile exists");
        assert_eq!(
            third.rectangle(),
            Rectangle::new(Point::new(10, 23), Size::new(4, 3))
        );
        assert_eq!(third.tile_top_left(), Point::new(10, 23));
    }
}
