#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "cyd_application_preview",
        "docs/assets/cyd_application_preview.png"
    )
)]
#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "linkage_blaze_gallery",
        "docs/assets/linkage_blaze_gallery.png"
    )
)]
//! Portable display and touch interfaces for Cheap Yellow Display (CYD)
//! applications.
//!
//! The [`Cyd`] trait represents a ready-to-use device with a display and
//! calibrated touch input. Hardware, browser, and in-memory devices all provide
//! the same [`Cyd`], [`CydDisplay`], [`CydTouch`], and
//! [`CydFrame`] interfaces.
//!
//! ## Portable CYD abstraction
//!
//! ```text
//! CydEsp / CydRp / CydWasm / CydMemory
//!                   │ implement
//!                   ▼
//!                  Cyd
//!           ┌───────┴───────┐
//!     parts().0         parts().1
//!     CydDisplay        CydTouch
//!          │                 │
//!  frame_mut()          try_read()
//!          ▼                 ▼
//!     CydFrame          TouchEvent
//!  borrowed frame       calibrated + oriented
//! ```
//!
//! [`Cyd::parts`] borrows the display and touch components together.
//! [`CydDisplay::frame_mut`] returns a temporary borrowed frame for a display
//! region, while [`CydTouch::try_read`] returns already calibrated and oriented
//! events. A full-screen frame is the special case where the borrowed region is
//! the complete display.
//!
//! > **Touch-event coordinates and drawing coordinates use the same logical
//! > orientation. Do not rotate touch points again.**
#![cfg_attr(
    not(feature = "doc-images"),
    doc = "\n> **Incomplete documentation preview:** Gallery images are omitted because the `doc-images` feature is disabled. From the workspace root, use `just docs` for authoritative local documentation.\n"
)]
#![doc = include_str!("../docs/cyd/gallery.md")]
#![doc = include_str!("../docs/cyd/application-example.md")]
#![doc = include_str!("../docs/cyd/drawing-strategies.md")]
#![doc = include_str!("../docs/cyd/implementations.md")]

// This must remain public because the ESP and RP platform implementations live
// in separate crates, but it is not part of the application-facing API.
#[doc(hidden)]
pub mod backend;
pub mod display;
pub mod touch;

use display::{ContiguousPixels, CydFrame};

/// Native panel width in pixels (landscape): 320. The CYD panel is fixed hardware.
pub(crate) const SCREEN_WIDTH: usize = 320;
/// Native panel height in pixels (landscape): 240. The CYD panel is fixed hardware.
pub(crate) const SCREEN_HEIGHT: usize = 240;
/// Total panel pixel count (`SCREEN_WIDTH * SCREEN_HEIGHT` = 320 * 240 = 76,800).
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

/// A ready-to-use CYD device with display and calibrated touch components.
///
/// [`Cyd::parts`] borrows both components together. The associated types retain
/// each implementation's concrete display, touch, and error types, while
/// generic application code can accept any `C: Cyd`.
///
/// The [module-level example](index.html#application-example) shows how to use
/// the display and calibrated touch input. To find the device's current
/// orientation, see [`Cyd::orientation`]. See the
/// [implementations](index.html#implementations-1) for `CydEsp`, `CydRp`,
/// `CydWasm`, and `CydMemory`.
pub trait Cyd: Sized {
    /// Error returned by both the display and calibrated touch parts.
    /// See the [application example](index.html#application-example) for a
    /// generic operation that returns this error.
    type Error;

    type Display: CydDisplay<Error = Self::Error>;
    type Touch: CydTouch<Error = Self::Error>;

    /// Borrow the display and calibrated touch components at once.
    ///
    /// The [application example](index.html#application-example) uses `parts`
    /// because its drawing loop needs both components simultaneously.
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    /// Borrow the display component.
    ///
    /// See the [application example](index.html#application-example) for drawing
    /// through a borrowed display component.
    fn display(&mut self) -> &mut Self::Display {
        self.parts().0
    }

    /// Borrow the calibrated touch component.
    ///
    /// See the [application example](index.html#application-example) for reading
    /// calibrated, oriented touch events through a borrowed touch component.
    fn touch(&mut self) -> &mut Self::Touch {
        self.parts().1
    }

    /// Return the logical orientation of this complete device.
    ///
    /// # Example
    ///
    /// For a device constructed in landscape orientation, compare the returned
    /// value directly and use it to obtain the application's logical display
    /// dimensions. Portrait orientations instead return 240×320.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{display::Orientation, Cyd};
    /// use embedded_graphics::prelude::Size;
    ///
    /// fn check_landscape_orientation<C: Cyd>(device: &C) {
    ///     let orientation = device.orientation();
    ///     assert_eq!(orientation, Orientation::Landscape);
    ///     assert_eq!(orientation.size(), Size::new(320, 240));
    /// }
    /// ```
    fn orientation(&self) -> Orientation;
}

/// A CYD touch source that returns calibrated, oriented touch events in logical
/// display coordinates.
pub trait CydTouch: Sized {
    /// Error returned when reading touch input.
    type Error;

    /// Try to read the next calibrated touch event without blocking.
    ///
    /// Returned points are calibrated and oriented into the same logical
    /// coordinates as the display's [`CydDisplay::screen_size`]. `Ok(Some(event))`
    /// means an event is available, `Ok(None)` means no event is available now,
    /// and `Err(error)` means the underlying touch source could not be read.
    ///
    /// The example below consumes the already oriented point directly;
    /// applications must not map it a second time.
    /// The [application example](index.html#application-example) shows this
    /// method in a complete read-and-draw flow.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{CydTouch, touch::TouchEvent};
    /// # use embedded_graphics::prelude::Point;
    /// # fn handle_point(_point: Point) {}
    ///
    /// fn read_calibrated<T: CydTouch>(touch: &mut T) -> Result<(), T::Error> {
    ///     if let Some(event) = touch.try_read()? {
    ///         match event {
    ///             TouchEvent::Down { point } | TouchEvent::Move { point } => {
    ///                 // `point` is already in logical display coordinates.
    ///                 handle_point(point);
    ///             }
    ///             TouchEvent::Up => {}
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn try_read(&mut self) -> Result<Option<TouchEvent>, Self::Error>;
}

/// A CYD display.
///
/// The screen is a fixed 320×240 RGB565 panel.
///
/// | Need | API | Reusable pixel-buffer storage |
/// | --- | --- | ---: |
/// | Normal drawing with enough RAM | [`full_frame_mut`](CydDisplay::full_frame_mut) | 153,600 bytes |
/// | Redraw one region | [`frame_mut`](CydDisplay::frame_mut) | 2 × rectangle pixel count bytes |
/// | Normal drawing with little RAM | [`for_each_tile`](CydDisplay::for_each_tile) | 2 × largest tile pixel count bytes |
/// | Existing or generated row-major RGB565 pixels | [`fill_contiguous`](CydDisplay::fill_contiguous) or [`fill_contiguous_full`](CydDisplay::fill_contiguous_full) | No reusable frame buffer |
/// | Small immediate [`DrawItem`](display::DrawItem) scene | [`draw_items`](CydDisplay::draw_items) | No pixel frame buffer |
///
/// The full-screen figure is `320 × 240 × 2` bytes for the fixed RGB565 panel.
/// `draw_items` does not need a pixel frame buffer, but it does need
/// allocation-free prepared-item capacity. Each nondegenerate `DrawItem`
/// consumes at most one prepared-item slot, so setting the capacity to the
/// number of supplied items is always safe. See [`CydDisplay::draw_items`] for
/// details.
///
/// Start with [`CydDisplay::full_frame_mut`] when a 153,600-byte frame buffer is
/// practical. The
/// [drawing-strategy guide](index.html#choose-a-drawing-strategy) compares
/// full-screen and regional buffering, tiled replay, and contiguous-pixel
/// streaming.
///
pub trait CydDisplay: backend::DisplayBackend {
    /// Screen size after applying the configured [`Orientation`]:
    /// 320×240 in landscape or 240×320 in portrait.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    ///
    /// # fn inspect(display: &impl CydDisplay) {
    /// let size = display.screen_size();
    /// assert!(
    ///     (size.width == 320 && size.height == 240)
    ///         || (size.width == 240 && size.height == 320)
    /// );
    /// # }
    /// ```
    fn screen_size(&self) -> Size;

    /// The device default background color.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    ///
    /// # fn inspect(display: &impl CydDisplay) {
    /// let background = display.background_color();
    /// let foreground = display.foreground_color();
    /// assert_eq!(display.background_565(), display.to_rgb565(background));
    /// assert_eq!(display.foreground_565(), display.to_rgb565(foreground));
    /// # }
    /// ```
    fn background_color(&self) -> Rgb888;

    /// The device default foreground/text color.
    ///
    /// See the [color getter example](CydDisplay::background_color).
    fn foreground_color(&self) -> Rgb888;

    /// The device default background color in the native `Rgb565` format.
    ///
    /// See the [color getter example](CydDisplay::background_color).
    fn background_565(&self) -> Rgb565;

    /// The device default foreground/text color in the native `Rgb565` format.
    ///
    /// See the [color getter example](CydDisplay::background_color).
    fn foreground_565(&self) -> Rgb565;

    /// Convert an `Rgb888` color to the device's native `Rgb565` format.
    ///
    /// See the [color getter example](CydDisplay::background_color).
    fn to_rgb565(&self, color: Rgb888) -> Rgb565 {
        rgb565_from_rgb888(color)
    }

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// See [`CydFrame`](display::CydFrame#coordinates-and-clipping) for the
    /// shared screen-coordinate and clipping model.
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
use embedded_graphics::{
    pixelcolor::{Rgb565, RgbColor},
    prelude::{Point, Size},
    primitives::Rectangle,
};

async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    let mut frame = display.frame_mut(Rectangle::new(
        Point::new(10, 10),
        Size::new(100, 40),
    ));
    frame.fill(Rgb565::BLUE).write_text("CYD").flush().await
}

# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{mono_font::ascii::FONT_9X15_BOLD, pixelcolor::Rgb888};
# let memory_cyd = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# let mut display = memory_cyd.display();
# futures_executor::block_on(draw(&mut display))?;
# if let Err(error) = assert_framebuffer_matches_expected_png(
#     &memory_cyd,
#     env!("CARGO_MANIFEST_DIR"),
#     "cyd_frame_mut_preview.png",
# ) {
#     panic!("{error}");
# }
# Ok::<(), device_envoy_core::memory::Error>(())
```

"#
    )]
    #[cfg_attr(
        all(feature = "host", feature = "doc-images"),
        doc = "\n![CYD frame preview][cyd_frame_mut_preview]\n"
    )]
    fn frame_mut(&mut self, rectangle: Rectangle) -> Self::Frame<'_> {
        backend::DisplayBackend::create_frame_mut(self, rectangle)
    }

    /// Borrow a full-screen frame, cleared to the device background color.
    ///
    /// See the [`Cyd` device-loop example](Cyd).
    fn full_frame_mut(&mut self) -> Self::Frame<'_> {
        self.frame_mut(Rectangle::new(Point::zero(), self.screen_size()))
    }

    /// Fill `rectangle` immediately with `color` in logical display coordinates.
    ///
    /// Unlike filling a frame returned by [`CydDisplay::frame_mut`], this is a
    /// device-level operation rather than a frame-buffered draw. Implementations
    /// clip to the logical display and treat an empty intersection as a no-op.
    ///
    /// The following example covers the immediate and contiguous operations:
    /// [`CydDisplay::fill_contiguous`], [`CydDisplay::draw_items`], [`CydDisplay::clear`],
    /// and [`CydDisplay::fill`].
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{CydDisplay, display::DrawItem};
    /// use embedded_graphics::{pixelcolor::{Rgb565, Rgb888}, prelude::{Point, RgbColor, Size}, primitives::Rectangle};
    ///
    /// async fn draw<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let rectangle = Rectangle::new(Point::zero(), Size::new(2, 2));
    ///     display.fill_rectangle(rectangle, Rgb565::BLACK)?;
    ///     display.fill_contiguous(rectangle, [Rgb565::RED; 4])?;
    ///     // One DrawItem, so reserve one prepared-item slot.
    ///     display.draw_items::<1>(rectangle, Rgb565::BLACK, [
    ///         DrawItem::Circle {
    ///             center: (1.0, 1.0), pixel_radius: 1.0, color: Rgb888::WHITE,
    ///         },
    ///     ])?;
    ///     display.clear()?;
    ///     display.fill(Rgb565::WHITE)
    /// }
    /// ```
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Self::Error>;

    /// Fill `rectangle` immediately from row-major native-color pixels.
    ///
    /// Empty rectangles are a no-op. Otherwise supply exactly
    /// `rectangle_pixel_count(rectangle)` pixels: a short iterator leaves the
    /// remaining pixels untouched, while extra pixels are ignored. This method
    /// does not infer missing pixels or repeat the final value.
    ///
    /// # Example
    ///
    /// Stream an image directly when its RGB565 pixels do not need further
    /// drawing or transformation:
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::{
    ///     CydDisplay,
    ///     display::{Image565Fixed, tga},
    /// };
    /// use embedded_graphics::{
    ///     prelude::Point,
    ///     primitives::Rectangle,
    /// };
    ///
    /// const BITMAP: Image565Fixed<45, 73, { 45 * 73 }> =
    ///     tga!(concat!(env!("CARGO_MANIFEST_DIR"),
    ///         "/docs/assets/cyd_fill_contiguous.tga"))
    ///     .to_565();
    ///
    /// fn stream_bitmap<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let bitmap = BITMAP.view();
    ///     let destination = Rectangle::new(Point::new(40, 30), bitmap.size());
    ///
    ///     display.fill_contiguous(destination, bitmap.rgb565_iter())
    /// }
    /// ```
    ///
    /// The `tga!` macro embeds and decodes the file at compile time. The view
    /// borrows that `const` image, supplies its dimensions, and yields pixels
    /// in row-major order. The destination can be anywhere on the display, and
    /// this path requires neither a frame buffer nor heap allocation.
    ///
    /// For a whole-screen bitmap, see the
    /// [`fill_contiguous_full` example](CydDisplay::fill_contiguous_full). See the
    /// [shared DNS tester's bitmap-streaming code](https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs#L377-L381)
    /// for a complete working example.
    #[cfg_attr(
        feature = "doc-images",
        doc = ::embed_doc_image::embed_image!(
            "cyd_fill_contiguous_preview",
            "docs/assets/cyd_fill_contiguous_preview.png"
        )
    )]
    #[cfg_attr(
        feature = "doc-images",
        doc = "\n![A bitmap streamed into a region of an in-memory CYD display.][cyd_fill_contiguous_preview]\n"
    )]
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
    /// # Example
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::CydDisplay;
    /// use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
    ///
    /// fn stream_background<D: CydDisplay>(display: &mut D) -> Result<(), D::Error> {
    ///     let screen_size = display.screen_size();
    ///     // A blue-green RGB565 gradient with a warmer lower-right corner.
    ///     let pixels = (0..screen_size.height).flat_map(|position_y| {
    ///         (0..screen_size.width).map(move |position_x| {
    ///             Rgb565::new(
    ///                 (position_x * 31 / (screen_size.width - 1)) as u8,
    ///                 (position_y * 63 / (screen_size.height - 1)) as u8,
    ///                 ((position_x + position_y) * 31
    ///                     / (screen_size.width + screen_size.height - 2)) as u8,
    ///             )
    ///         })
    ///     });
    ///     display.fill_contiguous_full(pixels)
    /// }
    /// ```
    ///
    /// The iterator generates each pixel just before it is sent, without a
    /// frame buffer or heap allocation. To position a stored bitmap, see the
    /// [`fill_contiguous` example](CydDisplay::fill_contiguous). The
    /// [Linkage Blaze clock](https://github.com/CarlKCarlK/linkage-blaze/blob/main/crates/linkage-blaze/src/examples/clock.rs#L148-L151)
    /// demonstrates full-screen streaming in a complete application.
    #[cfg_attr(
        feature = "doc-images",
        doc = ::embed_doc_image::embed_image!(
            "cyd_fill_contiguous_full_preview",
            "docs/assets/cyd_fill_contiguous_full_preview.png"
        )
    )]
    #[cfg_attr(
        feature = "doc-images",
        doc = "\n![A numerically generated gradient streamed into an in-memory CYD display.][cyd_fill_contiguous_full_preview]\n"
    )]
    fn fill_contiguous_full<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        self.fill_contiguous(Rectangle::new(Point::zero(), self.screen_size()), pixels)
    }

    /// Draw `items` immediately inside `bounds`.
    ///
    /// See the [immediate-operations example](CydDisplay::fill_rectangle) for a
    /// complete immediate-drawing flow.
    /// `DRAW_ITEM_CAPACITY` is the allocation-free capacity for prepared draw
    /// items. Each nondegenerate item consumes at most one slot, including an
    /// item that lies outside `bounds`. Using the total number of supplied items
    /// is always safe.
    ///
    /// # Panics
    ///
    /// Panics if preparing the items exhausts `DRAW_ITEM_CAPACITY`.
    fn draw_items<const DRAW_ITEM_CAPACITY: usize>(
        &mut self,
        bounds: Rectangle,
        background_color: Rgb565,
        items: impl IntoIterator<Item = display::DrawItem>,
    ) -> Result<(), Self::Error> {
        let bounds = bounds.intersection(&Rectangle::new(Point::zero(), self.screen_size()));
        let pixel_sources = ContiguousPixels::<DRAW_ITEM_CAPACITY>::from_draw_items(
            bounds,
            background_color,
            items,
        );
        self.fill_contiguous(pixel_sources.bounds(), pixel_sources.iter())
    }

    /// Clear the whole screen to the device default background color.
    ///
    /// New frames already start cleared to this color. This is for immediately
    /// returning the logical display to the default background between frame
    /// workflows.
    ///
    /// See the [immediate-operations example](CydDisplay::fill_rectangle).
    fn clear(&mut self) -> Result<(), Self::Error> {
        self.fill(self.background_565())
    }

    /// Fill the whole screen with an explicit color.
    ///
    /// See the [immediate-operations example](CydDisplay::fill_rectangle).
    fn fill(&mut self, color: Rgb565) -> Result<(), Self::Error> {
        self.fill_rectangle(Rectangle::new(Point::zero(), self.screen_size()), color)
    }

    /// Draw and flush each tile in `grid`.
    ///
    /// `draw` receives one frame for each tile. See
    /// [`CydFrame`](display::CydFrame#coordinates-and-clipping) for how the same
    /// screen-coordinate scene is clipped to each tile. Each frame is flushed
    /// after `draw` returns and before the next tile is processed. Only one tile
    /// is buffered at a time.
    ///
    /// See the [`TileGrid`](display::tiling::TileGrid) example for grid
    /// construction, buffer sizing, and a scene drawn across tile boundaries.
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
    use crate::cyd::display::{CydFrame, GetPixel};
    use crate::pixel_target::PixelTarget;
    use core::convert::Infallible;
    use embedded_graphics::pixelcolor::WebColors;
    use embedded_graphics::{
        Pixel,
        prelude::{Dimensions, DrawTarget},
    };

    // TODO The shared `linkage-blaze-cyd-memory` fake cannot replace this unit-test
    // double directly because a cyd-core <-> cyd-memory dev-dependency cycle gives
    // cyd-core's unit tests a second trait instance, so `CydMemory` no longer
    // implements *this* module's `CydDisplay` trait. Keep this tiny local
    // test double until the trait crate/test layout is refactored to break that cycle.
    struct TestCyd;

    struct TestFrame {
        rectangle: Rectangle,
    }

    impl backend::DisplayBackend for TestCyd {
        type Error = Infallible;
        type Frame<'a> = TestFrame;

        fn create_frame_mut(&mut self, rectangle: Rectangle) -> TestFrame {
            TestFrame { rectangle }
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

    impl Dimensions for TestFrame {
        fn bounding_box(&self) -> Rectangle {
            self.rectangle
        }
    }

    impl PixelTarget for TestFrame {
        fn set_pixel(&mut self, _point: Point, _color: Rgb565) {}
    }

    impl GetPixel for TestFrame {
        type Color = Rgb565;

        fn pixel(&self, _point: Point) -> Option<Self::Color> {
            None
        }
    }

    impl CydFrame for TestFrame {
        type Error = Infallible;

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
    fn tiled_frames_use_logical_display_rectangles() {
        let mut cyd = TestCyd;
        let grid = display::tiling::TileGrid::new(
            Rectangle::new(Point::new(10, 20), Size::new(8, 6)),
            2,
            2,
        );
        let mut tiles = display::tiling::Tiles::new(&mut cyd, grid);

        {
            let first = tiles.next().expect("first tile exists");
            assert_eq!(
                first.rectangle(),
                Rectangle::new(Point::new(10, 20), Size::new(4, 3))
            );
            assert_eq!(first.bounding_box(), first.rectangle());
        }

        {
            let second = tiles.next().expect("second tile exists");
            assert_eq!(
                second.rectangle(),
                Rectangle::new(Point::new(14, 20), Size::new(4, 3))
            );
            assert_eq!(second.bounding_box(), second.rectangle());
        }

        let third = tiles.next().expect("third tile exists");
        assert_eq!(
            third.rectangle(),
            Rectangle::new(Point::new(10, 23), Size::new(4, 3))
        );
        assert_eq!(third.bounding_box(), third.rectangle());
    }
}
