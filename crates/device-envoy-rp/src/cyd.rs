//! A device abstraction for a standalone 320x240 CYD-style SPI display/touch
//! module wired over SPI to a Raspberry Pi Pico (1 or 2).
//!
//! See [`CydRp`], [`CydRpUncalibrated`], and [`CydDisplayRp`] for the
//! primary constructors; the device-agnostic [`CydDisplay`],
//! [`CydTouch`], and [`CydTouchUncalibrated`] traits live in
//! [`device_envoy_core::cyd`]. The display uses `SPI0`; touch uses `SPI1`.

mod buffer;
mod display;
mod text;
#[path = "cyd/touch.rs"]
mod touch_driver;

use core::{convert::Infallible, fmt};

use device_envoy_core::button::Button;
use device_envoy_core::cyd::{
    SCREEN_PIXELS,
    display::{CydFrame, RectanglePixels},
    touch::{
        RawTouchEvent, TouchEvent,
        calibration::{CalibrationConfig, EnsureCalibrationOutcome, ensure_calibration},
    },
};
use device_envoy_core::pixel_target::PixelTarget;
// The device abstraction and its neutral support types live in
// `device-envoy-core::cyd`; re-export the public surface from this device crate.
pub use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydParts, CydTouch, CydTouchUncalibrated,
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
pub use display::{CydDisplayRpFlushError, CydDisplayRpInitError, DEFAULT_DISPLAY_SPI_HZ};
pub use text::DEFAULT_FONT;
pub use touch_driver::{CydTouchRpInitError, TOUCH_SPI_HZ};

use crate::flash_block::FlashBlockRp;
use display::CydDisplayRp as CydDisplayRpDevice;
use touch_driver::CydTouchRp as CydTouchRpDevice;

/// An owned CYD display component for RP boards.
pub struct CydDisplayRp {
    display: CydDisplayRpDevice,
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

/// An owned uncalibrated CYD touch component for RP boards.
pub struct CydTouchUncalibratedRp {
    touch: CydTouchRpDevice,
}

/// An owned calibrated CYD touch component for RP boards.
pub struct CydTouchRp {
    raw: CydTouchUncalibratedRp,
    calibration_config: CalibrationConfig,
}

/// A calibrated CYD RP bundle.
pub struct CydRp {
    /// The owned display component.
    pub display: CydDisplayRp,
    /// The owned calibrated touch component.
    pub touch: CydTouchRp,
}

/// An uncalibrated CYD RP bundle.
pub struct CydRpUncalibrated {
    /// The owned display component.
    pub display: CydDisplayRp,
    /// The owned uncalibrated touch component.
    pub touch: CydTouchUncalibratedRp,
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
/// The app chooses the pixel count (policy); [`CydDisplayRp::new`] owns the
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

/// A single in-progress frame backed by an `Rgb565` pixel buffer.
pub struct CydFrameRp<'a> {
    display: &'a mut CydDisplayRpDevice,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    // Default foreground color and font, copied from the owning `CydDisplayRp`, so
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
        self.raw_pixels_mut()[local_y * stride + local_x] = Rgb565::from(color).into_storage();
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
/// Error from a CYD RP display or touch operation.
pub enum CydError {
    /// Initializing the display over SPI failed.
    DisplayInit(CydDisplayRpInitError),
    /// Initializing the touch controller over SPI failed.
    TouchInit(CydTouchRpInitError),
    /// Flushing a frame to the display failed.
    DisplayFlush(CydDisplayRpFlushError),
}

impl CydDisplayRp {
    /// Construct a display-only `CydDisplayRp` that owns its draw buffer.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors CydEsp's constructor shape"
    )]
    pub fn new<const PIXEL_COUNT: usize, Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        statics: &'static CydStaticRp<PIXEL_COUNT>,
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
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
            display_spi_hz,
            orientation,
            background,
            foreground,
            font,
            pixel_buffer,
        )
    }

    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    fn new_inner<Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        display_spi: Peri<'static, SPI0>,
        display_sck_pin: Peri<'static, Sck>,
        display_mosi_pin: Peri<'static, Mosi>,
        display_miso_pin: Peri<'static, Miso>,
        display_cs_pin: Peri<'static, Cs>,
        display_dc_pin: Peri<'static, Dc>,
        display_rst_pin: Peri<'static, Rst>,
        display_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
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
        let mut display = CydDisplayRpDevice::new(
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            display_spi_hz,
            orientation,
        )?;
        let background565 = rgb565(background);
        display.fill(background565)?;

        Ok(Self {
            display,
            pixel_buffer,
            background,
            foreground,
            background565,
            foreground565: rgb565(foreground),
            font,
        })
    }
}

impl CydTouchUncalibratedRp {
    /// Construct an uncalibrated touch component.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub fn new<TouchSck, TouchMosi, TouchMiso, TouchCs, TouchIrq>(
        touch_spi: Peri<'static, SPI1>,
        touch_sck_pin: Peri<'static, TouchSck>,
        touch_mosi_pin: Peri<'static, TouchMosi>,
        touch_miso_pin: Peri<'static, TouchMiso>,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
    ) -> Result<Self, CydError>
    where
        TouchSck: Pin + ClkPin<SPI1>,
        TouchMosi: Pin + MosiPin<SPI1>,
        TouchMiso: Pin + MisoPin<SPI1>,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        Ok(Self {
            touch: CydTouchRpDevice::new(
                touch_spi,
                touch_sck_pin,
                touch_mosi_pin,
                touch_miso_pin,
                touch_cs_pin,
                touch_irq_pin,
            )?,
        })
    }
}

impl CydRp {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    pub const SCREEN_PIXELS: usize = SCREEN_PIXELS;

    /// Create [`CydStaticRp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticRp<PIXEL_COUNT> {
        CydStaticRp::new()
    }

    /// Construct a calibrated CYD bundle using the saved-or-interactive calibration flow.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub async fn new<
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
        display_spi_hz: u32,
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
        calibration_flash_block: &mut FlashBlockRp,
        recalibration_button: &mut impl Button,
        confirmed_message: Option<&str>,
    ) -> crate::Result<(Self, EnsureCalibrationOutcome)>
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
        let CydRpUncalibrated { mut display, touch } = CydRpUncalibrated::new(
            statics,
            display_spi,
            display_sck_pin,
            display_mosi_pin,
            display_miso_pin,
            display_cs_pin,
            display_dc_pin,
            display_rst_pin,
            display_backlight_pin,
            display_spi_hz,
            orientation,
            background,
            foreground,
            font,
            touch_spi,
            touch_sck_pin,
            touch_mosi_pin,
            touch_miso_pin,
            touch_cs_pin,
            touch_irq_pin,
        )?;
        let (touch, ensure_calibration_outcome) = ensure_calibration(
            &mut display,
            touch,
            calibration_flash_block,
            recalibration_button,
            confirmed_message,
        )
        .await
        .map_err(|error| match error.kind {
            device_envoy_core::cyd::touch::calibration::EnsureCalibrationErrorKind::Device(
                cyd_error,
            ) => crate::Error::from(cyd_error),
            device_envoy_core::cyd::touch::calibration::EnsureCalibrationErrorKind::Flash(
                flash_error,
            ) => flash_error,
        })?;
        Ok((Self { display, touch }, ensure_calibration_outcome))
    }
}

impl Cyd for CydRp {
    type Error = CydError;
    type Display = CydDisplayRp;
    type Touch = CydTouchRp;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }
}

impl CydParts for CydRp {
    fn into_parts(self) -> (Self::Display, Self::Touch) {
        let Self { display, touch } = self;
        (display, touch)
    }

    fn from_parts(display: Self::Display, touch: Self::Touch) -> Self {
        Self { display, touch }
    }
}

impl CydRpUncalibrated {
    /// Construct an uncalibrated CYD bundle from display and touch parts.
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
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
        display_spi_hz: u32,
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
        Ok(Self {
            display: CydDisplayRp::new(
                statics,
                display_spi,
                display_sck_pin,
                display_mosi_pin,
                display_miso_pin,
                display_cs_pin,
                display_dc_pin,
                display_rst_pin,
                display_backlight_pin,
                display_spi_hz,
                orientation,
                background,
                foreground,
                font,
            )?,
            touch: CydTouchUncalibratedRp::new(
                touch_spi,
                touch_sck_pin,
                touch_mosi_pin,
                touch_miso_pin,
                touch_cs_pin,
                touch_irq_pin,
            )?,
        })
    }
}

fn rgb565(color: Rgb888) -> Rgb565 {
    Rgb565::from(color)
}

impl fmt::Debug for CydDisplayRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydDisplayRp")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydTouchUncalibratedRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchUncalibratedRp")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydTouchRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchRp")
            .field("calibration_config", &self.calibration_config)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydRp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CydRp").finish_non_exhaustive()
    }
}

impl fmt::Debug for CydRpUncalibrated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydRpUncalibrated")
            .finish_non_exhaustive()
    }
}

impl CydDisplay for CydDisplayRp {
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
            &mut *self.pixel_buffer,
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

impl CydTouchUncalibrated for CydTouchUncalibratedRp {
    type Error = CydError;
    type Calibrated = CydTouchRp;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(self.touch.read_raw_touch_event())
    }

    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated {
        CydTouchRp {
            raw: self,
            calibration_config,
        }
    }
}

impl CydTouch for CydTouchRp {
    type Error = CydError;
    type Uncalibrated = CydTouchUncalibratedRp;

    fn read(&mut self) -> Result<Option<TouchEvent>, CydError> {
        Ok(self
            .raw
            .touch
            .read_raw_touch_event()
            .map(|raw_touch_event| match raw_touch_event {
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
            }))
    }

    fn calibration_config(&self) -> CalibrationConfig {
        self.calibration_config
    }

    fn decalibrate(self) -> Self::Uncalibrated {
        self.raw
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
