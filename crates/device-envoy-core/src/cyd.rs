//! A device abstraction for the "Cheap Yellow Display" (CYD) with touch.
//!
//! CYD boards pair an ILI9341 display with an XPT2046 resistive touch
//! controller. The root keeps the device model itself: [`Cyd`] splits into
//! [`CydDisplay`] and [`CydTouch`], with display-side support types in
//! [`display`] and touch-side support types in [`touch`].
//!
//! See [`Cyd`] for the primary trait and usage example.
//!

pub mod display;
pub mod touch;

use display::ContiguousPixels;

/// Native panel width in pixels (landscape): 320. The CYD panel is fixed hardware.
pub(crate) const SCREEN_WIDTH: usize = 320;
/// Native panel height in pixels (landscape): 240. The CYD panel is fixed hardware.
pub(crate) const SCREEN_HEIGHT: usize = 240;
/// Total panel pixel count (`SCREEN_WIDTH * SCREEN_HEIGHT` = 76,800).
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

use core::convert::Infallible;

use crate::pixel_target::{PixelTarget, rgb565_from_rgb888};
use embedded_graphics::{
    pixelcolor::{Rgb565, Rgb888, raw::RawU16},
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};

/// Error type used by a CYD device or frame.
///
/// This marker lets downstream generic examples distinguish device/flush errors
/// from their own local errors when using `?`.
pub trait CydFlushError {}

/// Device/flush error for CYD implementations whose presentation path cannot fail.
impl CydFlushError for Infallible {}

/// A complete CYD device that offers display and touch parts.
///
/// ```rust,no_run
/// # use core::{convert::Infallible, future::ready};
/// # use device_envoy_core::cyd::{Cyd, CydDisplay, CydTouch};
/// # use device_envoy_core::cyd::display::CydFrame;
/// # use device_envoy_core::cyd::touch::TouchEvent;
/// # use device_envoy_core::pixel_target::PixelTarget;
/// # use embedded_graphics::{
/// #     pixelcolor::{Rgb565, Rgb888},
/// #     prelude::{DrawTarget, OriginDimensions, Point, RgbColor, Size},
/// #     primitives::Rectangle,
/// # };
/// # struct DemoCyd;
/// # struct DemoDisplay;
/// # struct DemoTouch;
/// # struct DemoFrame;
/// # impl Cyd for DemoCyd {
/// #     type Error = Infallible;
/// #     type Display<'a> = DemoDisplay;
/// #     type Touch<'a> = DemoTouch;
/// #     fn parts(&mut self) -> (Self::Display<'_>, Self::Touch<'_>) {
/// #         (DemoDisplay, DemoTouch)
/// #     }
/// # }
/// # impl CydDisplay for DemoDisplay {
/// #     type Error = Infallible;
/// #     type Frame<'a> = DemoFrame;
/// #     fn screen_size(&self) -> Size { Size::new(320, 240) }
/// #     fn background(&self) -> Rgb888 { Rgb888::BLACK }
/// #     fn foreground(&self) -> Rgb888 { Rgb888::WHITE }
/// #     fn background_565(&self) -> Rgb565 { Rgb565::BLACK }
/// #     fn foreground_565(&self) -> Rgb565 { Rgb565::WHITE }
/// #     fn frame_mut_with_tile_top_left(
/// #         &mut self,
/// #         rectangle: Rectangle,
/// #         _tile_top_left: Point,
/// #     ) -> Self::Frame<'_> {
/// #         let _ = rectangle;
/// #         DemoFrame
/// #     }
/// #     fn fill_rectangle(
/// #         &mut self,
/// #         _rectangle: Rectangle,
/// #         _color: Rgb565,
/// #     ) -> Result<(), Self::Error> {
/// #         Ok(())
/// #     }
/// #     fn fill_contiguous<I>(
/// #         &mut self,
/// #         _rectangle: Rectangle,
/// #         _pixels: I,
/// #     ) -> Result<(), Self::Error>
/// #     where
/// #         I: IntoIterator<Item = Rgb565>,
/// #     {
/// #         Ok(())
/// #     }
/// # }
/// # impl DrawTarget for DemoFrame {
/// #     type Color = Rgb565;
/// #     type Error = Infallible;
/// #     fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
/// #     where
/// #         I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
/// #     {
/// #         Ok(())
/// #     }
/// # }
/// # impl OriginDimensions for DemoFrame {
/// #     fn size(&self) -> Size { Size::new(320, 240) }
/// # }
/// # impl PixelTarget for DemoFrame {
/// #     fn width(&self) -> usize { 320 }
/// #     fn height(&self) -> usize { 240 }
/// #     fn put_pixel(&mut self, _x: usize, _y: usize, _color: Rgb888) {}
/// # }
/// # impl CydFrame for DemoFrame {
/// #     type Error = Infallible;
/// #     fn rectangle(&self) -> Rectangle {
/// #         Rectangle::new(Point::zero(), Size::new(320, 240))
/// #     }
/// #     fn fill(&mut self, _color: Rgb565) -> &mut Self { self }
/// #     fn write_text(&mut self, _text: &str) -> &mut Self { self }
/// #     fn copy_from_565(&mut self, _src: &[u16]) -> device_envoy_core::Result<()> {
/// #         Ok(())
/// #     }
/// #     fn flush(&mut self) -> impl core::future::Future<Output = Result<(), <Self as CydFrame>::Error>> {
/// #         ready(Ok(()))
/// #     }
/// # }
/// # impl CydTouch for DemoTouch {
/// #     type Error = Infallible;
/// #     fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error> { Ok(None) }
/// # }
/// # async fn draw(cyd: &mut impl Cyd<Error = Infallible>) -> Result<(), Infallible> {
/// let (mut display, mut touch) = cyd.parts();
/// let mut frame = display.full_frame_mut();
/// frame.write_text("Hello CYD").flush().await?;
/// let _touch_event = touch.read()?;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(
    feature = "host",
    doc = "\nHost-side test double: [`crate::memory::MemoryCyd`]."
)]
#[cfg_attr(
    feature = "wasm",
    doc = "\nBrowser-simulated device: [`crate::wasm::CydWasm`]."
)]
pub trait Cyd {
    /// Error returned when flushing a frame or reading touch fails.
    type Error: CydFlushError;

    /// Display part offered by this device.
    type Display<'a>: CydDisplay<Error = Self::Error>
    where
        Self: 'a;

    /// Touch part offered by this device.
    type Touch<'a>: CydTouch<Error = Self::Error>
    where
        Self: 'a;

    /// Borrow display and touch as independent parts.
    fn parts(&mut self) -> (Self::Display<'_>, Self::Touch<'_>);
}

/// A CYD display: hands out cleared, rectangle-sized 2D frames.
///
/// The screen is a fixed 320x240 RGB565 panel. `CydDisplay` offers three
/// ways to draw, trading memory for flexibility: [`display::CydFrame`]s that can be
/// drawn into and flushed to any rectangle on screen; tiled frames (see
/// [`CydDisplay::tiles`]) that cover the screen in smaller pieces when memory
/// is tight; and contiguous-pixel methods (see
/// [`CydDisplay::fill_contiguous`]) that stream pixels straight to the screen
/// with virtually no buffering.
pub trait CydDisplay {
    /// Error returned when flushing a frame fails.
    type Error: CydFlushError;

    /// The per-rectangle frame type this device produces.
    ///
    /// Its [`display::CydFrame::Error`] is pinned to this display's [`CydDisplay::Error`], so
    /// `frame.flush().await?` in generic code propagates a single
    /// `S::Error`.
    type Frame<'a>: display::CydFrame<Error = Self::Error>
    where
        Self: 'a;

    /// Oriented screen size for the configured orientation.
    fn screen_size(&self) -> Size;

    /// The device default background color.
    fn background(&self) -> Rgb888;

    /// The device default foreground/text color.
    fn foreground(&self) -> Rgb888;

    /// The device default background color in the native `Rgb565` format.
    fn background_565(&self) -> Rgb565;

    /// The device default foreground/text color in the native `Rgb565` format.
    fn foreground_565(&self) -> Rgb565;

    /// Convert an `Rgb888` color to the device's native `Rgb565` format.
    fn to_rgb565(&self, color: Rgb888) -> Rgb565 {
        rgb565_from_rgb888(color)
    }

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// Drawing commands are interpreted in screen coordinates:
    /// `tile_top_left` is subtracted before pixels are written into the
    /// frame-local buffer. Regular, non-tiled frames use `(0, 0)` and therefore
    /// draw in frame-local coordinates.
    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> Self::Frame<'_>;

    /// Borrow a frame covering `rectangle`, cleared to the device background color.
    ///
    /// The frame remembers its `rectangle`, so [`display::CydFrame::flush`] presents it
    /// at the rectangle's top-left with no separate position argument.
    fn frame_mut(&mut self, rectangle: Rectangle) -> Self::Frame<'_> {
        self.frame_mut_with_tile_top_left(rectangle, Point::zero())
    }

    /// Borrow a full-screen frame, cleared to the device background color.
    fn full_frame_mut(&mut self) -> Self::Frame<'_> {
        self.frame_mut(Rectangle::new(Point::zero(), self.screen_size()))
    }

    /// Fill `rectangle` immediately in physical-screen coordinates.
    ///
    /// Unlike [`display::CydFrame::fill`], this is a device-level operation rather than a
    /// frame-local buffered draw. Implementations clip to the physical screen and
    /// treat an empty intersection as a no-op.
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), Self::Error>;

    /// Fill `rectangle` immediately from row-major native-color pixels.
    ///
    /// Empty rectangles are a no-op.
    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>;

    /// Present a native-color rectangle buffer at `top_left`.
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
    fn clear(&mut self) -> Result<(), Self::Error> {
        self.fill(self.background_565())
    }

    /// Fill the whole screen with an explicit color.
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
    /// # use device_envoy_core::cyd::CydDisplay;
    /// # use device_envoy_core::cyd::display::{CydFrame, tiling::TileGrid};
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

/// A CYD touch source for calibrated, screen-space events that apps read.
///
/// [`CydTouch::read`] returns a [`touch::TouchEvent`] carrying an x-y point in the
/// same screen coordinates as the display, or `None` when there is no touch.
pub trait CydTouch {
    /// Error returned when reading touch fails.
    type Error: CydFlushError;

    /// Read the next calibrated, screen-space touch event, if any.
    ///
    /// Returns `Ok(None)` when there is no pending touch (including devices
    /// constructed without touch). Errors only on a hardware/read failure.
    fn read(&mut self) -> Result<Option<touch::TouchEvent>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cyd::display::CydFrame;
    use embedded_graphics::pixelcolor::WebColors;
    use embedded_graphics::{Pixel, prelude::OriginDimensions};

    // TODO The shared `linkage-blaze-cyd-memory` fake cannot replace this unit-test
    // double directly because a cyd-core <-> cyd-memory dev-dependency cycle gives
    // cyd-core's unit tests a second trait instance, so `MemoryCyd` no longer
    // implements *this* module's `Cyd`/`CydDisplay` traits. Keep this tiny local
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
