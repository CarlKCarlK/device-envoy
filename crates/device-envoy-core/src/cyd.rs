//! A device abstraction for the "Cheap Yellow Display" (CYD) display and touch parts.
//!
//! Tested on so far:
//!
//! - an integrated ESP32 Cheap Yellow Display board
//! - the same standalone CYD 320x240 SPI display/touch board, driven
//!   externally by both ESP32 and Raspberry Pi Pico 2 setups
//!
//! See [`Cyd`] for the primary trait and usage example.

pub mod display;
pub mod touch;

use display::ContiguousPixels;

/// Native panel width in pixels (landscape): 320. The CYD panel is fixed hardware.
pub(crate) const SCREEN_WIDTH: usize = 320;
/// Native panel height in pixels (landscape): 240. The CYD panel is fixed hardware.
pub(crate) const SCREEN_HEIGHT: usize = 240;
/// Total panel pixel count (`SCREEN_WIDTH * SCREEN_HEIGHT` = 76,800).
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

use crate::pixel_target::rgb565_from_rgb888;
use embedded_graphics::{
    pixelcolor::{Rgb565, Rgb888, raw::RawU16},
    prelude::{Point, Size},
    primitives::Rectangle,
};

use display::Orientation;
use touch::{RawTouchEvent, TouchEvent, calibration::CalibrationConfig};

/// A device abstraction for the "Cheap Yellow Display" (CYD) display and touch parts.
///
/// `Cyd` is the core trait for ready-to-use device bundles. It provides borrowed access
/// to calibrated display and touch halves. Generic app code should use `Cyd` to remain
/// compatible with hardware designs where display and touch share underlying resources.
///
/// For backends that support owned deconstruction and reassembly, see [`CydParts`].
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("cyd_trait_preview", "docs/assets/cyd_trait_preview.png")
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

Implementations include the in-memory mock [`CydMemory`](crate::memory::CydMemory), the browser-simulated [`CydWasm`](crate::wasm::CydWasm), and platform crates for ESP32 and Pico boards.

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
# let (mut display, mut touch) = Cyd::parts(&mut cyd);
// Create a pixel-buffer covering the whole screen that starts filled with background color.
let mut frame = display.full_frame_mut();

frame.write_text("Hello CYD");
// An app would usually run this in a loop: read touch, draw, flush, repeat.
if let Some(TouchEvent::Down { point } | TouchEvent::Move { point }) = touch.read()? {
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
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
# })?;
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```

![CYD trait preview][cyd_trait_preview]
"#
)]
pub trait Cyd: Sized {
    /// Error returned by both the display and calibrated touch parts.
    type Error;

    /// The owned display component stored by this device.
    type Display: CydDisplay<Error = Self::Error>;

    /// The owned calibrated touch component stored by this device.
    type Touch: CydTouch<Error = Self::Error>;

    /// Borrow both calibrated halves at once.
    ///
    /// See the [`Cyd`] trait documentation for a usage example.
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    /// Return the logical orientation of the bundled display and touch parts.
    ///
    /// The default uses the oriented screen dimensions. It intentionally
    /// treats equal-sized inverted presentations as their non-inverted
    /// orientation; applications that preserve 180-degree inversion should
    /// override this method with their stored orientation.
    fn orientation(&mut self) -> Orientation {
        let screen_size = self.display().screen_size();
        if screen_size.width > screen_size.height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }

    /// Borrow the display half.
    ///
    #[cfg_attr(
        feature = "host",
        doc = r#"

```rust
use device_envoy_core::cyd::{Cyd, CydDisplay};
use embedded_graphics::pixelcolor::Rgb888;
# use device_envoy_core::memory::CydMemory;
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     prelude::{RgbColor, Size},
# };
# let mut cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
let display = Cyd::display(&mut cyd);
assert_eq!(display.screen_size(), Size::new(320, 240));
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```
"#
    )]
    fn display(&mut self) -> &mut Self::Display {
        self.parts().0
    }

    /// Borrow the calibrated touch half.
    ///
    #[cfg_attr(
        feature = "host",
        doc = r#"

```rust
use device_envoy_core::cyd::{Cyd, CydTouch, touch::TouchEvent};
use embedded_graphics::pixelcolor::Rgb888;
# use device_envoy_core::memory::CydMemory;
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     prelude::{Point, RgbColor, Size},
# };
# let mut cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
cyd.push_touch_event(TouchEvent::Down { point: Point::new(160, 120) });
assert!(matches!(
    Cyd::touch(&mut cyd).read()?,
    Some(TouchEvent::Down { point }) if point == Point::new(160, 120)
));
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```
"#
    )]
    fn touch(&mut self) -> &mut Self::Touch {
        self.parts().1
    }
}

/// Extension trait for device implementations that support owned deconstruction and reassembly.
///
/// Some `Cyd` implementations can cleanly split into independently-owned display and touch
/// parts, then be reassembled from those parts. For example, backends with two independent
/// SPI peripherals or reference-counted shared state.
///
/// Backends that share a single bus (like one-SPI hardware) cannot guarantee that the
/// split parts remain valid in isolation, so they do not implement `CydParts`.
///
/// Generic code that only needs borrowed access should depend on [`Cyd`], not `CydParts`.
/// Use `CydParts` only when ownership-level transitions are required, such as in test
/// harnesses or calibration flows.
///
/// See [`CydParts::into_parts`] for a round-trip example.
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::cyd::{
    Cyd, CydParts, CydDisplay, CydTouch,
    display::CydFrame,
    touch::TouchEvent,
};
use embedded_graphics::pixelcolor::Rgb888;
# use device_envoy_core::memory::CydMemory;
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
let mut cyd = CydMemory::from_parts(cyd.display().to_owned(), cyd.touch().to_owned());
cyd.push_touch_event(TouchEvent::Down { point: Point::new(12, 34) });
let (mut display, mut touch) = cyd.into_parts();
assert_eq!(display.screen_size(), Size::new(320, 240));
assert!(matches!(
    touch.read()?,
    Some(TouchEvent::Down { point }) if point == Point::new(12, 34)
));

let cyd = CydMemory::from_parts(display, touch);
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
# })?;
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```
"#
)]
pub trait CydParts: Cyd {
    /// Consume the device into its owned calibrated halves.
    ///
    /// The parts must come from the same underlying device. On shared-state backends like
    /// `CydMemory` and `CydWasm`, mismatched pairings cannot be detected.
    fn into_parts(self) -> (Self::Display, Self::Touch);

    /// Reassemble a device from its owned calibrated halves.
    ///
    /// The parts must come from the same underlying device. On shared-state backends like
    /// `CydMemory` and `CydWasm`, mismatched pairings cannot be detected.
    ///
    /// See [`CydParts::into_parts`] for a round-trip example.
    fn from_parts(display: Self::Display, touch: Self::Touch) -> Self;
}

/// A raw-touch source that can run the shared calibration flow and become calibrated.
pub trait CydTouchUncalibrated: Sized {
    /// Error returned when reading raw touch fails.
    type Error;
    type Calibrated: CydTouch<Error = Self::Error, Uncalibrated = Self>;

    /// Read the next raw touch event, if any.
    ///
    /// This bypasses any active [`touch::TouchEvent`] calibration mapping and
    /// exists specifically for the shared calibration driver. See the
    /// [touch calibration module documentation](touch::calibration) for usage.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Apply `calibration_config`, becoming a calibrated touch source.
    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated;
}

/// A CYD touch source for calibrated, screen-space events that apps read.
///
/// [`CydTouch::read`] returns a [`touch::TouchEvent`] carrying an x-y point in
/// the same screen coordinates as the display, or `None` when there is no
/// touch.
pub trait CydTouch: Sized {
    /// Error returned when reading touch fails.
    type Error;
    type Uncalibrated: CydTouchUncalibrated<Error = Self::Error, Calibrated = Self>;

    /// Read the next calibrated touch event, if any.
    ///
    /// Returned points use fixed landscape-panel coordinates (`320x240`),
    /// regardless of display orientation. Consumers that render an oriented
    /// screen must apply [`Orientation::map_landscape_point`] exactly once
    /// before hit testing. Returns `Ok(None)` when there is no pending touch.
    /// Errors only on a hardware/read failure. See the [`Cyd`] trait
    /// documentation for a usage example.
    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error>;

    fn calibration_config(&self) -> CalibrationConfig;

    /// Discard the calibration, becoming an uncalibrated touch source.
    fn decalibrate(self) -> Self::Uncalibrated;
}

/// A CYD display.
///
/// The screen is a fixed 320x240 RGB565 panel. `CydDisplay` offers three
/// ways to draw, trading memory for flexibility: [`display::CydFrame`]s that can be
/// drawn into and flushed to any rectangle on screen; tiled frames (see
/// [`CydDisplay::tiles`]) that cover the screen (or a rectangle) in smaller pieces when memory
/// is tight; and contiguous-pixel methods (see
/// [`CydDisplay::fill_contiguous`]) that stream pixels straight to the screen
/// with virtually no buffering.
///
pub trait CydDisplay {
    /// Error returned when flushing a frame fails.
    type Error;

    /// The per-rectangle frame type this device produces.
    ///
    /// Its [`display::CydFrame::Error`] is pinned to this display's [`CydDisplay::Error`], so
    /// `frame.flush().await?` in generic code propagates a single
    /// `S::Error`.
    type Frame<'a>: display::CydFrame<Error = Self::Error>
    where
        Self: 'a;

    /// Oriented screen size for the configured orientation.
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
assert_eq!(display.background_565(), display.to_rgb565(display.background()));
assert_eq!(display.foreground_565(), display.to_rgb565(display.foreground()));
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```
"#
    )]
    fn screen_size(&self) -> Size;

    /// The device default background color.
    ///
    /// See [`CydDisplay::screen_size`] for an example covering the device getter family.
    fn background(&self) -> Rgb888;

    /// The device default foreground/text color.
    ///
    /// See [`CydDisplay::screen_size`] for an example covering the device getter family.
    fn foreground(&self) -> Rgb888;

    /// The device default background color in the native `Rgb565` format.
    ///
    /// See [`CydDisplay::screen_size`] for an example covering the device getter family.
    fn background_565(&self) -> Rgb565;

    /// The device default foreground/text color in the native `Rgb565` format.
    ///
    /// See [`CydDisplay::screen_size`] for an example covering the device getter family.
    fn foreground_565(&self) -> Rgb565;

    /// Convert an `Rgb888` color to the device's native `Rgb565` format.
    ///
    /// See [`CydDisplay::screen_size`] for an example covering the device getter family.
    fn to_rgb565(&self, color: Rgb888) -> Rgb565 {
        rgb565_from_rgb888(color)
    }

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// Drawing commands are interpreted in screen coordinates:
    /// `tile_top_left` is subtracted before pixels are written into the
    /// frame-local buffer. Regular, non-tiled frames use `(0, 0)` and therefore
    /// draw in frame-local coordinates.
    ///
    #[cfg_attr(
        feature = "doc-images",
        doc = ::embed_doc_image::embed_image!(
            "cyd_frame_mut_with_tile_top_left_preview",
            "docs/assets/cyd_frame_mut_with_tile_top_left_preview.png"
        )
    )]
    #[cfg_attr(
        feature = "host",
        doc = r#"

```rust
use device_envoy_core::cyd::{CydDisplay, display::CydFrame};
use device_envoy_core::UnwrapInfallible;
use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
use embedded_graphics::{
    Drawable,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyle, Rectangle},
};
# use embedded_graphics::mono_font::ascii::FONT_9X15_BOLD;
# futures_executor::block_on(async {
# let memory_cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# let mut display = memory_cyd.display();
let mut frame = display.frame_mut_with_tile_top_left(
    Rectangle::new(Point::new(32, 24), Size::new(48, 32)),
    Point::new(32, 24),
);
frame.fill(Rgb565::GREEN);
Rectangle::new(Point::new(36, 28), Size::new(6, 6))
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(&mut frame)
    .unwrap_infallible();
Rectangle::new(Point::new(70, 46), Size::new(6, 6))
    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
    .draw(&mut frame)
    .unwrap_infallible();
frame.flush().await?;
# if let Err(error) = assert_framebuffer_matches_expected_png(
#     &memory_cyd,
#     env!("CARGO_MANIFEST_DIR"),
#     "cyd_frame_mut_with_tile_top_left_preview.png",
# ) {
#     panic!("{error}");
# }
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
# })?;
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```

![CYD tiled frame preview][cyd_frame_mut_with_tile_top_left_preview]
"#
    )]
    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_>;

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// The frame remembers its `rectangle`, so [`display::CydFrame::flush`] presents it
    /// at the rectangle's top-left with no separate position argument.
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
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
# })?;
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```

![CYD frame preview][cyd_frame_mut_preview]
"#
    )]
    fn frame_mut(&mut self, rectangle: Rectangle) -> Self::Frame<'_> {
        self.frame_mut_with_tile_top_left(rectangle, Point::zero())
    }

    /// Borrow a full-screen frame, cleared to the device background color.
    ///
    /// See the [`Cyd`] trait documentation for a usage example.
    fn full_frame_mut(&mut self) -> Self::Frame<'_> {
        self.frame_mut(Rectangle::new(Point::zero(), self.screen_size()))
    }

    /// Fill `rectangle` immediately in physical-screen coordinates.
    ///
    /// Unlike [`display::CydFrame::fill`], this is a device-level operation rather than a
    /// frame-local buffered draw. Implementations clip to the physical screen and
    /// treat an empty intersection as a no-op.
    ///
    /// See the [CydDisplay trait documentation](Self) for related drawing APIs.
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Self::Error>;

    /// Fill `rectangle` immediately from row-major native-color pixels.
    ///
    /// Empty rectangles are a no-op.
    ///
    /// See the [CydDisplay trait documentation](Self) for related drawing APIs.
    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>;

    /// Fill the entire screen immediately from row-major native-color pixels.
    ///
    /// This is the full-screen convenience form of [`CydDisplay::fill_contiguous`].
    /// Empty pixel iterators are allowed; implementations retain the same size
    /// validation behavior as [`CydDisplay::fill_contiguous`].
    fn fill_contiguous_full<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        self.fill_contiguous(Rectangle::new(Point::zero(), self.screen_size()), pixels)
    }

    /// Present a native-color rectangle buffer at `top_left`.
    ///
    /// See the [CydDisplay trait documentation](Self) for related drawing APIs.
    fn flush_at(
        &mut self,
        buffer: &impl display::RectanglePixels,
        top_left: Point,
    ) -> Result<(), Self::Error> {
        let rectangle = Rectangle::new(
            top_left,
            Size::new(buffer.width() as u32, buffer.height() as u32),
        );
        self.fill_contiguous(
            rectangle,
            buffer
                .raw_pixels()
                .iter()
                .copied()
                .map(|pixel| Rgb565::from(RawU16::new(pixel))),
        )
    }

    /// Draw projected draw items immediately inside `bounds`.
    ///
    /// See the [display module documentation](display) for the draw-item types
    /// this consumes.
    fn draw_items<const PIXEL_SOURCE_COUNT: usize>(
        &mut self,
        bounds: Rectangle,
        background: Rgb565,
        items: impl IntoIterator<Item = display::DrawItem>,
    ) -> Result<(), Self::Error> {
        let bounds = bounds.intersection(&Rectangle::new(Point::zero(), self.screen_size()));
        let pixel_sources =
            ContiguousPixels::<PIXEL_SOURCE_COUNT>::from_draw_items(bounds, background, items);
        self.fill_contiguous(pixel_sources.bounds(), pixel_sources.iter())
    }

    /// Clear the whole screen to the device default background color.
    ///
    /// New frames already start cleared to this color. This is for immediately
    /// returning the physical screen to the default background between frame
    /// workflows.
    ///
    /// See the [CydDisplay trait documentation](Self) for related drawing APIs.
    fn clear(&mut self) -> Result<(), Self::Error> {
        self.fill(self.background_565())
    }

    /// Fill the whole screen with an explicit color.
    ///
    /// See the [CydDisplay trait documentation](Self) for related drawing APIs.
    fn fill(&mut self, color: Rgb565) -> Result<(), Self::Error> {
        self.fill_rectangle(Rectangle::new(Point::zero(), self.screen_size()), color)
    }

    /// Drive `grid` as a sequence of low-memory tiles.
    ///
    /// The returned [`Tiles`](display::tiling::Tiles) is a lending/streaming iterator (it does not
    /// implement [`Iterator`], because each yielded frame borrows the device's
    /// single reusable frame buffer). Each yielded frame draws in screen
    /// coordinates via each frame's non-zero [`display::CydFrame::tile_top_left`], and is
    /// presented with [`display::CydFrame::flush`]:
    ///
    /// ```rust,no_run
    /// # use device_envoy_core::cyd::{CydDisplay, display::{CydFrame, tiling::TileGrid}};
    /// # async fn draw<D: CydDisplay>(display: &mut D, grid: TileGrid) -> Result<(), D::Error> {
    /// let mut tiles = display.tiles(grid);
    /// while let Some(mut frame) = tiles.next() {
    ///     // draw into `frame` in screen coordinates...
    ///     frame.flush().await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn tiles(&mut self, grid: display::tiling::TileGrid) -> display::tiling::Tiles<'_, Self>
    where
        Self: Sized,
    {
        display::tiling::Tiles::new(self, grid)
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

    impl CydDisplay for TestCyd {
        type Error = Infallible;
        type Frame<'a> = TestFrame;

        fn screen_size(&self) -> Size {
            Size::new(320, 240)
        }

        fn background(&self) -> Rgb888 {
            Rgb888::CSS_BLACK
        }

        fn foreground(&self) -> Rgb888 {
            Rgb888::CSS_WHITE
        }

        fn background_565(&self) -> Rgb565 {
            self.to_rgb565(self.background())
        }

        fn foreground_565(&self) -> Rgb565 {
            self.to_rgb565(self.foreground())
        }

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
        let mut tiles = cyd.tiles(grid);

        let first = tiles.next().expect("first tile exists");
        assert_eq!(
            first.rectangle(),
            Rectangle::new(Point::new(10, 20), Size::new(4, 3))
        );
        assert_eq!(first.tile_top_left(), Point::new(10, 20));
        drop(first);

        let second = tiles.next().expect("second tile exists");
        assert_eq!(
            second.rectangle(),
            Rectangle::new(Point::new(14, 20), Size::new(4, 3))
        );
        assert_eq!(second.tile_top_left(), Point::new(14, 20));
        drop(second);

        let third = tiles.next().expect("third tile exists");
        assert_eq!(
            third.rectangle(),
            Rectangle::new(Point::new(10, 23), Size::new(4, 3))
        );
        assert_eq!(third.tile_top_left(), Point::new(10, 23));
    }
}
