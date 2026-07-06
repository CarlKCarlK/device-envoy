//! A device abstraction for a standalone 320x240 ILI9341 + XPT2046
//! resistive-touch module wired over SPI to a Raspberry Pi Pico (1 or 2).
//!
//! See [`CydRp`] for the primary constructor and usage example; the
//! device-agnostic `Cyd`/`CydDisplay`/`CydTouch` traits it implements live in
//! [`device_envoy_core::cyd`]. The display uses `SPI0`; touch uses `SPI1`.

mod buffer;
mod display;
mod text;
#[path = "cyd/touch.rs"]
mod touch_driver;

use core::{convert::Infallible, fmt};

use device_envoy_core::cyd::{
    CydIoError, SCREEN_PIXELS,
    display::{CydFrame, RectanglePixels},
    touch::{CydRawTouch, RawTouchEvent, TouchEvent, calibration::CalibrationConfig},
};
use device_envoy_core::pixel_target::PixelTarget;
// The device abstraction and its neutral support types live in
// `device-envoy-core::cyd`; re-export the public surface from this device crate.
pub use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch,
    display::{Orientation, tiling},
    touch,
};
use embassy_rp::Peri;
use embassy_rp::gpio::Pin;
use embassy_rp::peripherals::{SPI0, SPI1};
use embassy_rp::spi::{ClkPin, MisoPin, MosiPin};
use embedded_graphics::{
    Pixel,
    mono_font::MonoFont,
    pixelcolor::{IntoStorage, Rgb565, Rgb888},
    prelude::{Dimensions, DrawTarget, OriginDimensions, Point, Size},
    primitives::Rectangle,
};
use static_cell::StaticCell;

use buffer::DynPixelBuffer;
pub use buffer::{PixelBuffer, RegionBuffer, RegionView};
pub use display::{CydDisplayRpFlushError, CydDisplayRpInitError, DISPLAY_SPI_HZ};
pub use text::DEFAULT_FONT;
pub use touch_driver::{CydTouchRpInitError, TOUCH_SPI_HZ};

use display::CydDisplayRp;
use touch_driver::CydTouchRp;

/// A standalone 320x240 ILI9341 + XPT2046 module wired over SPI to a Pico.
pub struct CydRp {
    display: CydDisplayRp,
    touch: Option<CydTouchRp>,
    calibration_config: Option<CalibrationConfig>,
    // Every CydRp owns exactly one draw buffer. Apps that don't draw through it
    // pass a zero-sized buffer (e.g. `CydStaticRp<0>`).
    pixel_buffer: &'static mut dyn DynPixelBuffer,
    // Default drawing style. Background clears the device at construction and
    // fills every new frame; foreground and font drive `CydFrameRp::write_text`.
    // The `Rgb565` versions are precomputed so the hot drawing paths skip the
    // per-call conversion.
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// Static storage for a [`CydRp`]-owned pixel buffer.
///
/// The app declares one at file scope and names the workspace pixel count it
/// wants:
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::cyd::{CydRp, CydStaticRp};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// static CYD_STATIC: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
/// ```
///
/// The app chooses the pixel count (policy); [`CydRp::new_display_only`] owns the
/// initialization protocol and the storage details.
pub struct CydStaticRp<const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
}

impl<const PIXEL_COUNT: usize> CydStaticRp<PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydRp::new_static`] so all
    /// construction goes through the `CydRp` device abstraction.
    pub(crate) const fn new() -> Self {
        Self {
            pixel_buffer: StaticCell::new(),
        }
    }
}

/// A [`CydRp`] whose touch calibration is confirmed present.
pub struct CalibratedCydRp<'a> {
    cyd: &'a mut CydRp,
    calibration_config: CalibrationConfig,
}

/// The display half of a [`CydRp`], borrowed from [`Cyd::parts`].
pub struct CydDisplayRpPart<'a> {
    display: &'a mut CydDisplayRp,
    pixel_buffer: &'a mut dyn DynPixelBuffer,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// The touch half of a [`CydRp`], borrowed from [`Cyd::parts`].
pub struct CydTouchRpPart<'a> {
    touch: Option<&'a mut CydTouchRp>,
    calibration_config: Option<CalibrationConfig>,
}

/// A single in-progress frame backed by an `Rgb565` pixel buffer.
pub struct CydFrameRp<'a> {
    display: &'a mut CydDisplayRp,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    // Default foreground color and font, copied from the owning `CydRp`, so
    // `write_text` can render with the device default style.
    pub(crate) foreground565: Rgb565,
    pub(crate) font: &'static MonoFont<'static>,
}

impl<'a> CydFrameRp<'a> {
    /// Borrow the frame's underlying pixel view.
    pub fn view_mut(&mut self) -> &mut RegionView<'a> {
        &mut self.view
    }

    /// Fill the frame with an explicit color.
    pub fn fill(&mut self, color: Rgb565) -> &mut Self {
        self.view.fill(color);
        self
    }

    /// The frame's width in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.view.width()
    }

    /// The frame's height in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.view.height()
    }

    /// Borrow the frame's raw RGB565 pixels, row-major.
    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.view.raw_pixels_mut()
    }

    /// Present this frame's pixels at its rectangle's top-left (set by
    /// [`CydDisplay::frame_mut`]).
    pub fn flush(&mut self) -> Result<(), CydError> {
        Ok(self
            .display
            .flush_buffer(&self.view, self.rectangle.top_left)?)
    }

    fn local_x(&self, x: i32) -> Option<usize> {
        usize::try_from(x.checked_sub(self.tile_top_left.x)?).ok()
    }

    fn local_y(&self, y: i32) -> Option<usize> {
        usize::try_from(y.checked_sub(self.tile_top_left.y)?).ok()
    }
}

impl DrawTarget for CydFrameRp<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color);
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            let Some(local_x) = self.local_x(point.x) else {
                continue;
            };
            let Some(local_y) = self.local_y(point.y) else {
                continue;
            };
            if local_x < self.view.width() && local_y < self.view.height() {
                let index = local_y * self.view.width() + local_x;
                self.raw_pixels_mut()[index] = color.into_storage();
            }
        }
        Ok(())
    }
}

impl Dimensions for CydFrameRp<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.view.size())
    }
}

impl PixelTarget for CydFrameRp<'_> {
    fn width(&self) -> usize {
        usize::try_from(self.tile_top_left.x)
            .expect("tile top-left x must be non-negative")
            .checked_add(self.width())
            .expect("frame width must fit in usize")
    }

    fn height(&self) -> usize {
        usize::try_from(self.tile_top_left.y)
            .expect("tile top-left y must be non-negative")
            .checked_add(self.height())
            .expect("frame height must fit in usize")
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= self.view.width() || local_y >= self.view.height() {
            return;
        }
        let stride = self.view.width();
        self.raw_pixels_mut()[local_y * stride + local_x] = CydRp::rgb565(color).into_storage();
    }

    /// The frame buffer already stores RGB565, so a decoded image pixel can be
    /// written verbatim with no RGB888 round-trip.
    fn put_pixel_565(&mut self, x: usize, y: usize, rgb565: u16) {
        let Some(local_x) = self.local_x(x as i32) else {
            return;
        };
        let Some(local_y) = self.local_y(y as i32) else {
            return;
        };
        if local_x >= self.view.width() || local_y >= self.view.height() {
            return;
        }
        let stride = self.view.width();
        self.raw_pixels_mut()[local_y * stride + local_x] = rgb565;
    }
}

#[derive(Debug, derive_more::From)]
/// Error from a `CydRp` device, frame, or touch operation.
pub enum CydError {
    /// Reading or saving calibration to flash failed.
    Flash(crate::Error),
    /// Initializing the display over SPI failed.
    DisplayInit(CydDisplayRpInitError),
    /// Initializing the touch controller over SPI failed.
    TouchInit(CydTouchRpInitError),
    /// Flushing a frame to the display failed.
    DisplayFlush(CydDisplayRpFlushError),
    /// No touch controller is attached to this device.
    TouchUnavailable,
    /// No calibration has been set on this device.
    CalibrationUnavailable,
}

impl CydIoError for CydError {}

impl CydRp {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    pub const SCREEN_PIXELS: usize = SCREEN_PIXELS;

    /// Create [`CydStaticRp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    ///
    /// Equivalent to `CydStaticRp::<PIXEL_COUNT>::new()` but namespaced under `CydRp` so
    /// all construction calls share a common prefix.
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// use device_envoy_rp::cyd::{CydRp, CydStaticRp};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    ///
    /// static CYD_STATIC: CydStaticRp<{ CydRp::SCREEN_PIXELS }> = CydRp::new_static();
    /// ```
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticRp<PIXEL_COUNT> {
        CydStaticRp::new()
    }

    #[inline]
    /// Convert an `Rgb888` color to the device's native `Rgb565` format.
    pub fn rgb565(color: Rgb888) -> Rgb565 {
        Rgb565::from(color)
    }

    /// Construct a display-only `CydRp` (no touch) that owns its draw buffer,
    /// initializing the buffer from app-provided [`CydStaticRp`] storage.
    ///
    /// The app picks the size via `PIXEL_COUNT`; `CydRp` owns the init protocol. Use
    /// [`CydDisplay::frame_mut`] or [`CydDisplay::full_frame_mut`] to render into and flush the owned buffer.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors CydEsp's constructor shape"
    )]
    pub fn new_display_only<const PIXEL_COUNT: usize, Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, CydError>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);
        Self::new_inner(
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            orientation,
            background,
            foreground,
            font,
            None,
            pixel_buffer,
        )
    }

    /// Construct a full `CydRp` with touch that owns its draw buffer.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors CydEsp's constructor shape"
    )]
    pub fn new<
        const PIXEL_COUNT: usize,
        Sck,
        Mosi,
        Miso,
        Cs,
        Dc,
        Rst,
        Backlight,
        TouchSck,
        TouchMosi,
        TouchMiso,
        TouchCs,
        TouchIrq,
    >(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: Peri<'static, SPI1>,
        touch_sck_pin: Peri<'static, TouchSck>,
        touch_mosi_pin: Peri<'static, TouchMosi>,
        touch_miso_pin: Peri<'static, TouchMiso>,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
    ) -> Result<Self, CydError>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
        TouchSck: Pin + ClkPin<SPI1>,
        TouchMosi: Pin + MosiPin<SPI1>,
        TouchMiso: Pin + MisoPin<SPI1>,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        let touch = CydTouchRp::new(
            touch_spi,
            touch_sck_pin,
            touch_mosi_pin,
            touch_miso_pin,
            touch_cs_pin,
            touch_irq_pin,
        )?;
        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);

        Self::new_inner(
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            orientation,
            background,
            foreground,
            font,
            Some(touch),
            pixel_buffer,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors CydEsp's constructor shape"
    )]
    fn new_inner<Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        touch: Option<CydTouchRp>,
        pixel_buffer: &'static mut dyn DynPixelBuffer,
    ) -> Result<Self, CydError>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let mut display = CydDisplayRp::new(
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            orientation,
        )?;
        // Start every device on a clean background so apps never see boot-time
        // garbage before their first draw.
        let background565 = Self::rgb565(background);
        display.fill(background565)?;

        Ok(Self {
            display,
            touch,
            calibration_config: None,
            pixel_buffer,
            background,
            foreground,
            background565,
            foreground565: Self::rgb565(foreground),
            font,
        })
    }

    /// The device's current touch calibration, if any.
    #[must_use]
    pub fn calibration_config(&self) -> Option<CalibrationConfig> {
        self.calibration_config
    }

    /// Clear the device's touch calibration.
    pub fn clear_calibration(&mut self) {
        self.calibration_config = None;
    }

    /// Set the device's touch calibration.
    pub fn set_calibration(&mut self, calibration_config: CalibrationConfig) {
        self.calibration_config = Some(calibration_config);
    }

    /// Borrow this device as calibrated, or fail if no calibration is set.
    pub fn ensure_calibration(&mut self) -> Result<CalibratedCydRp<'_>, CydError> {
        let calibration_config = self
            .calibration_config
            .ok_or(CydError::CalibrationUnavailable)?;

        Ok(CalibratedCydRp {
            cyd: self,
            calibration_config,
        })
    }

    /// Read the next raw (uncalibrated) touch event, if any.
    pub fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, CydError> {
        let touch = self.touch.as_mut().ok_or(CydError::TouchUnavailable)?;
        Ok(touch.read_raw_touch_event())
    }
}

impl CalibratedCydRp<'_> {
    /// Clear the underlying device's touch calibration.
    pub fn clear_calibration(&mut self) {
        self.cyd.clear_calibration();
    }

    /// Read the next calibrated touch event, if any.
    pub fn read(&mut self) -> Result<Option<TouchEvent>, CydError> {
        let raw_touch_event = self
            .cyd
            .touch
            .as_mut()
            .ok_or(CydError::TouchUnavailable)?
            .read_raw_touch_event();

        Ok(
            raw_touch_event.map(|raw_touch_event| match raw_touch_event {
                RawTouchEvent::Down { raw_x, raw_y } => {
                    let (x, y) = self.calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Down {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Move { raw_x, raw_y } => {
                    let (x, y) = self.calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Move {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Up => TouchEvent::Up,
            }),
        )
    }
}

impl CydRawTouch for CydRp {
    type Error = CydError;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, CydError> {
        CydRp::read_raw_touch_event(self)
    }
}

impl fmt::Debug for CydRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CydRp").finish_non_exhaustive()
    }
}

// ── Device-agnostic `Cyd` trait impls ─────────────────────────────────────────
//
// These let platform-neutral code (`device-envoy-core::cyd` consumers) drive
// the concrete rp `CydRp` through the `Cyd`/`CydFrame` traits without naming
// any rp type.

impl Cyd for CydRp {
    type Error = CydError;
    type Display<'a> = CydDisplayRpPart<'a>;
    type Touch<'a> = CydTouchRpPart<'a>;

    #[inline]
    fn parts(&mut self) -> (CydDisplayRpPart<'_>, CydTouchRpPart<'_>) {
        (
            CydDisplayRpPart {
                display: &mut self.display,
                pixel_buffer: &mut *self.pixel_buffer,
                background: self.background,
                foreground: self.foreground,
                background565: self.background565,
                foreground565: self.foreground565,
                font: self.font,
            },
            CydTouchRpPart {
                touch: self.touch.as_mut(),
                calibration_config: self.calibration_config,
            },
        )
    }
}

impl CydDisplay for CydDisplayRpPart<'_> {
    type Error = CydError;
    type Frame<'a>
        = CydFrameRp<'a>
    where
        Self: 'a;

    #[inline]
    fn screen_size(&self) -> Size {
        self.display.size()
    }

    fn background(&self) -> Rgb888 {
        self.background
    }

    fn foreground(&self) -> Rgb888 {
        self.foreground
    }

    fn background_565(&self) -> Rgb565 {
        self.background565
    }

    fn foreground_565(&self) -> Rgb565 {
        self.foreground565
    }

    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: Rectangle,
        tile_top_left: Point,
    ) -> CydFrameRp<'_> {
        self.display.make_frame_with_tile_top_left(
            self.pixel_buffer,
            rectangle,
            tile_top_left,
            self.background565,
            self.foreground565,
            self.font,
        )
    }

    #[inline]
    fn fill_rectangle(&mut self, rectangle: Rectangle, color: Rgb565) -> Result<(), CydError> {
        Ok(self.display.fill_rectangle(rectangle, color)?)
    }

    #[inline]
    fn fill_contiguous<I>(&mut self, rectangle: Rectangle, pixels: I) -> Result<(), CydError>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        Ok(self.display.fill_contiguous(rectangle, pixels)?)
    }

    #[inline]
    fn flush_at(&mut self, buffer: &impl RectanglePixels, top_left: Point) -> Result<(), CydError> {
        Ok(self.display.flush_buffer(buffer, top_left)?)
    }
}

impl Cyd for CalibratedCydRp<'_> {
    type Error = CydError;
    type Display<'a>
        = CydDisplayRpPart<'a>
    where
        Self: 'a;
    type Touch<'a>
        = CydTouchRpPart<'a>
    where
        Self: 'a;

    #[inline]
    fn parts(&mut self) -> (CydDisplayRpPart<'_>, CydTouchRpPart<'_>) {
        let cyd = &mut *self.cyd;
        (
            CydDisplayRpPart {
                display: &mut cyd.display,
                pixel_buffer: &mut *cyd.pixel_buffer,
                background: cyd.background,
                foreground: cyd.foreground,
                background565: cyd.background565,
                foreground565: cyd.foreground565,
                font: cyd.font,
            },
            CydTouchRpPart {
                touch: cyd.touch.as_mut(),
                calibration_config: Some(self.calibration_config),
            },
        )
    }
}

impl CydTouch for CydTouchRpPart<'_> {
    type Error = CydError;

    fn read(&mut self) -> Result<Option<TouchEvent>, CydError> {
        let Some(calibration_config) = self.calibration_config else {
            return Ok(None);
        };
        let Some(touch) = self.touch.as_mut() else {
            return Ok(None);
        };
        Ok(touch
            .read_raw_touch_event()
            .map(|raw_touch_event| match raw_touch_event {
                RawTouchEvent::Down { raw_x, raw_y } => {
                    let (x, y) = calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Down {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Move { raw_x, raw_y } => {
                    let (x, y) = calibration_config.map_raw_to_screen(raw_x, raw_y);
                    TouchEvent::Move {
                        point: Point::new(x as i32, y as i32),
                    }
                }
                RawTouchEvent::Up => TouchEvent::Up,
            }))
    }
}

impl CydFrame for CydFrameRp<'_> {
    type Error = CydError;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameRp::fill(self, color)
    }

    fn write_text(&mut self, text: &str) -> &mut Self {
        CydFrameRp::write_text(self, text)
    }

    fn copy_from_565(&mut self, src: &[u16]) -> device_envoy_core::Result<()> {
        let dst = self.raw_pixels_mut();
        if dst.len() != src.len() {
            return Err(device_envoy_core::Error::CopySize {
                src_len: src.len(),
                frame_len: dst.len(),
            });
        }
        dst.copy_from_slice(src);
        Ok(())
    }

    // Flushing the panel over SPI is synchronous, so this future resolves on its
    // first poll. The `async fn` is the device-agnostic frame boundary the
    // render loop awaits; on the MCU it adds no suspension.
    async fn flush(&mut self) -> Result<(), CydError> {
        CydFrameRp::flush(self)
    }
}
