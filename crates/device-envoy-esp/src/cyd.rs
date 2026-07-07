//! A device abstraction for the "Cheap Yellow Display" (CYD) family of
//! 320x240 ILI9341 + XPT2046 resistive-touch ESP32 boards.
//!
//! See [`CydEsp`], [`CydEspUncalibrated`], and [`CydDisplayEsp`] for the
//! primary constructors; the device-agnostic [`CydDisplay`],
//! [`CydTouch`], and [`CydTouchUncalibrated`] traits live in
//! [`device_envoy_core::cyd`].

mod buffer;
mod display;
mod text;
#[path = "cyd/touch.rs"]
mod touch_driver;

use core::{convert::Infallible, fmt};

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
use device_envoy_core::cyd::{
    SCREEN_PIXELS,
    display::{CydFrame, RectanglePixels},
    touch::{RawTouchEvent, TouchEvent, calibration::CalibrationConfig},
};
use device_envoy_core::pixel_target::PixelTarget;
pub use display::{CydDisplayEspFlushError, CydDisplayEspInitError, DISPLAY_SPI_HZ};
// The device abstraction and its neutral support types live in
// `device-envoy-core::cyd`; re-export the public surface from this device crate.
pub use device_envoy_core::cyd::{
    CydDisplay, CydTouch, CydTouchUncalibrated,
    display::{Orientation, tiling},
    touch,
};
pub use text::DEFAULT_FONT;
pub use touch_driver::{CydTouchEspInitError, TOUCH_SPI_HZ};

use display::CydDisplayEsp as CydDisplayEspDevice;
use touch_driver::CydTouchEsp as CydTouchEspDevice;

/// An owned CYD-family ESP32 display component.
pub struct CydDisplayEsp {
    display: CydDisplayEspDevice,
    // Every CydEsp owns exactly one draw buffer. Apps that don't draw through it
    // pass a zero-sized buffer (e.g. `CydStaticEsp<0>`).
    pixel_buffer: &'static mut dyn DynPixelBuffer,
    // Default drawing style. Background clears the device at construction and
    // fills every new frame; foreground and font drive `CydFrameEsp::write_text`.
    // The `Rgb565` versions are precomputed so the hot drawing paths skip the
    // per-call conversion.
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// An owned uncalibrated CYD-family ESP32 touch component.
pub struct CydTouchUncalibratedEsp {
    touch: CydTouchEspDevice,
}

/// An owned calibrated CYD-family ESP32 touch component.
pub struct CydTouchEsp {
    raw: CydTouchUncalibratedEsp,
    calibration_config: CalibrationConfig,
}

/// A calibrated CYD-family ESP32 bundle.
pub struct CydEsp {
    pub display: CydDisplayEsp,
    pub touch: CydTouchEsp,
}

/// An uncalibrated CYD-family ESP32 bundle.
pub struct CydEspUncalibrated {
    pub display: CydDisplayEsp,
    pub touch: CydTouchUncalibratedEsp,
}

/// Static storage for a [`CydEsp`]-owned pixel buffer.
///
/// The app declares one at file scope and names the workspace pixel count it
/// wants:
///
/// ```rust,no_run
/// static CYD_STATIC: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
/// ```
///
/// The app chooses the pixel count (policy); [`CydDisplayEsp::new`] owns the
/// initialization protocol and the storage details.
pub struct CydStaticEsp<const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
}

impl<const PIXEL_COUNT: usize> CydStaticEsp<PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydEsp::new_static`] so all
    /// construction goes through the `CydEsp` device abstraction.
    pub(crate) const fn new() -> Self {
        Self {
            pixel_buffer: StaticCell::new(),
        }
    }
}

/// A single in-progress frame backed by an `Rgb565` pixel buffer.
pub struct CydFrameEsp<'a> {
    display: &'a mut CydDisplayEspDevice,
    view: RegionView<'a>,
    // Where this frame presents and how large it is: set from the `Rectangle`
    // passed to `frame_mut`, so `flush` needs no separate position argument.
    rectangle: Rectangle,
    // Tile top-left in screen coordinates. Drawing coordinates are translated
    // by this point before reaching the local frame buffer.
    tile_top_left: Point,
    // Default foreground color and font, copied from the owning `CydDisplayEsp`, so
    // `write_text` can render with the device default style.
    pub(crate) foreground565: Rgb565,
    pub(crate) font: &'static MonoFont<'static>,
}

impl<'a> CydFrameEsp<'a> {
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

impl DrawTarget for CydFrameEsp<'_> {
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

impl Dimensions for CydFrameEsp<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(self.tile_top_left, self.view.size())
    }
}

impl PixelTarget for CydFrameEsp<'_> {
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

/// Error from a CYD ESP display or touch operation.
#[derive(Debug, derive_more::From)]
pub enum CydError {
    /// Initializing the display over SPI failed.
    DisplayInit(CydDisplayEspInitError),
    /// Initializing the touch controller over SPI failed.
    TouchInit(CydTouchEspInitError),
    /// Flushing a frame to the display failed.
    DisplayFlush(CydDisplayEspFlushError),
}

impl CydDisplayEsp {
    /// Construct a display-only CYD display component that owns its draw buffer.
    pub fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, CydError> {
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
            pixel_buffer,
        )
    }

    fn new_inner(
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        pixel_buffer: &'static mut dyn DynPixelBuffer,
    ) -> Result<Self, CydError> {
        let mut display = CydDisplayEspDevice::new(
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

impl CydTouchUncalibratedEsp {
    pub fn new(
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    ) -> Result<Self, CydError> {
        Ok(Self {
            touch: CydTouchEspDevice::new(
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

impl CydEsp {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    pub const SCREEN_PIXELS: usize = SCREEN_PIXELS;

    /// Create [`CydStaticEsp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT> {
        CydStaticEsp::new()
    }

    pub fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
        calibration_config: CalibrationConfig,
    ) -> Result<Self, CydError> {
        let display = CydDisplayEsp::new(
            statics,
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
        )?;
        let touch = CydTouchUncalibratedEsp::new(
            touch_spi,
            touch_sck_pin,
            touch_mosi_pin,
            touch_miso_pin,
            touch_cs_pin,
            touch_irq_pin,
        )?
        .calibrate(calibration_config);
        Ok(Self { display, touch })
    }
}

impl CydEspUncalibrated {
    pub fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        display_spi: impl esp_hal::spi::master::Instance + 'static,
        display_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        display_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        display_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        display_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        touch_spi: impl esp_hal::spi::master::Instance + 'static,
        touch_sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        touch_miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    ) -> Result<Self, CydError> {
        Ok(Self {
            display: CydDisplayEsp::new(
                statics,
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
            )?,
            touch: CydTouchUncalibratedEsp::new(
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

impl fmt::Debug for CydDisplayEsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydDisplayEsp")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydTouchUncalibratedEsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchUncalibratedEsp")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydTouchEsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydTouchEsp")
            .field("calibration_config", &self.calibration_config)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CydEsp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CydEsp").finish_non_exhaustive()
    }
}

impl fmt::Debug for CydEspUncalibrated {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydEspUncalibrated")
            .finish_non_exhaustive()
    }
}

impl CydDisplay for CydDisplayEsp {
    type Error = CydError;
    type Frame<'a>
        = CydFrameEsp<'a>
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
    ) -> CydFrameEsp<'_> {
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

impl CydTouchUncalibrated for CydTouchUncalibratedEsp {
    type Error = CydError;
    type Calibrated = CydTouchEsp;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(self.touch.read_raw_touch_event())
    }

    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated {
        CydTouchEsp {
            raw: self,
            calibration_config,
        }
    }
}

impl CydTouch for CydTouchEsp {
    type Error = CydError;
    type Uncalibrated = CydTouchUncalibratedEsp;

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

impl CydFrame for CydFrameEsp<'_> {
    type Error = CydError;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, color: Rgb565) -> &mut Self {
        CydFrameEsp::fill(self, color)
    }

    fn write_text(&mut self, text: &str) -> &mut Self {
        CydFrameEsp::write_text(self, text)
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
        CydFrameEsp::flush(self)
    }
}
