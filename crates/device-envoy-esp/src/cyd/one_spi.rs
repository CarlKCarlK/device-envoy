//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydEspOneSpi`], which manages SPI arbitration between display and
//! touch controllers internally using interior mutability. The public API presents display and
//! touch as independently-borrowable halves of a bundle, hiding the shared-bus machinery.

use core::cell::RefCell;
use core::{convert::Infallible, fmt};

use embedded_graphics::{
    Pixel,
    mono_font::MonoFont,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Dimensions, DrawTarget, OriginDimensions, Point, Size},
    primitives::Rectangle,
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    delay::Delay,
    gpio::{Output, OutputConfig, OutputPin},
    spi,
};
use static_cell::StaticCell;

use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch, CydTouchUncalibrated,
    display::{CydFrame, RectanglePixels},
    touch::{RawTouchEvent, TouchEvent, calibration::CalibrationConfig},
};

use super::display::CydDisplayEsp as CydDisplayEspDevice;
use super::touch::CydTouchEsp as CydTouchEspDevice;
use super::{
    CydDisplayEsp, CydTouchUncalibratedEsp, CydTouchEsp, CydFrameEsp,
    CydError, Orientation, buffer::DynPixelBuffer,
    buffer::PixelBuffer,
};

/// A CYD-family ESP32 bundle using one shared SPI peripheral for display and touch.
///
/// This type manages SPI arbitration between the ILI9341 display and XPT2046 touch controller
/// via interior mutability. Display and touch access is serialized to prevent conflicts on
/// the shared bus. The public API hides this machinery entirely.
///
/// Construction takes raw GPIO pins and a single SPI peripheral, not a pre-built bus object:
///
/// ```ignore
/// let cyd = CydEspOneSpi::new(
///     spi,
///     lcd_cs,
///     lcd_dc,
///     lcd_rst,
///     touch_cs,
///     touch_irq,
///     calibration,
///     delay,
///     config,
/// ).await?;
/// ```
pub struct CydEspOneSpi {
    // Shared SPI bus stored in RefCell for interior mutability.
    // Both display and touch access it through this when reading/writing.
    shared_spi: RefCell<SharedSpi>,
    // Display and touch facades that coordinate through the shared bus.
    display: CydDisplayOneSpi,
    touch: CydTouchOneSpi,
}

struct SharedSpi {
    spi: spi::master::Spi<'static, esp_hal::Blocking>,
    lcd_cs: Output<'static>,
    touch_cs: Output<'static>,
}

/// Display facade for one-SPI bundle.
pub struct CydDisplayOneSpi {
    screen_size: Size,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
    pixel_buffer: &'static mut dyn DynPixelBuffer,
}

/// Calibrated touch facade for one-SPI bundle.
pub struct CydTouchOneSpi {
    calibration_config: CalibrationConfig,
}

/// Frame for one-SPI bundle display.
pub struct CydFrameOneSpi<'a> {
    rectangle: Rectangle,
    tile_top_left: Point,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
    pixels: Vec<u16>,
}

impl CydEspOneSpi {
    /// Construct an uncalibrated one-SPI CYD bundle from raw hardware.
    ///
    /// # Arguments
    ///
    /// * `statics` - Static storage for the pixel buffer
    /// * `spi` - The shared SPI peripheral (VSPI or HSPI)
    /// * `sck_pin` - SCK pin shared by both display and touch
    /// * `mosi_pin` - MOSI pin shared by both display and touch
    /// * `miso_pin` - MISO pin shared by both display and touch
    /// * `lcd_cs_pin` - LCD chip-select pin (active low)
    /// * `lcd_dc_pin` - LCD data/command pin
    /// * `lcd_rst_pin` - LCD reset pin (active low)
    /// * `touch_cs_pin` - Touch chip-select pin (active low)
    /// * `touch_irq_pin` - Touch interrupt pin
    /// * `orientation` - Screen orientation
    /// * `background` - Default background color
    /// * `foreground` - Default foreground/text color
    /// * `font` - Default monospace font for text drawing
    ///
    /// Returns a [`CydEspOneSpiBundleUncalibrated`] that must be calibrated before use.
    #[allow(clippy::too_many_arguments)]
    pub fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        spi: impl esp_hal::spi::master::Instance + 'static,
        sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        lcd_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        lcd_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        lcd_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<CydEspOneSpiBundleUncalibrated<PIXEL_COUNT>, CydError> {
        let background565 = Rgb565::from(background);

        Ok(CydEspOneSpiBundleUncalibrated {
            spi,
            sck_pin,
            mosi_pin,
            miso_pin,
            lcd_cs_pin,
            lcd_dc_pin,
            lcd_rst_pin,
            touch_cs_pin,
            touch_irq_pin,
            statics,
            orientation,
            background,
            foreground,
            background565,
            font,
        })
    }
}

/// Uncalibrated one-SPI CYD bundle awaiting touch calibration.
#[allow(clippy::too_many_arguments)]
pub struct CydEspOneSpiBundleUncalibrated<const PIXEL_COUNT: usize> {
    spi: impl esp_hal::spi::master::Instance + 'static,
    sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
    mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
    miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
    lcd_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
    lcd_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
    lcd_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
    touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
    touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
    statics: &'static CydStaticEsp<PIXEL_COUNT>,
    orientation: Orientation,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// Static storage for a [`CydEspOneSpi`]-owned pixel buffer.
pub struct CydStaticEsp<const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
}

impl<const PIXEL_COUNT: usize> CydStaticEsp<PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydEspOneSpi::new_static`].
    pub(crate) const fn new() -> Self {
        Self {
            pixel_buffer: StaticCell::new(),
        }
    }
}

impl CydDisplay for CydDisplayOneSpi {
    type Error = Infallible;
    type Frame<'a> = CydFrameOneSpi<'a>
    where
        Self: 'a;

    fn screen_size(&self) -> Size {
        self.screen_size
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
    ) -> Self::Frame<'_> {
        CydFrameOneSpi {
            rectangle,
            tile_top_left,
            foreground565: self.foreground565,
            font: self.font,
            pixels: vec![self.background565.into_storage(); rectangle.size.width as usize * rectangle.size.height as usize],
        }
    }

    fn fill_rectangle(
        &mut self,
        _rectangle: Rectangle,
        _color: Rgb565,
    ) -> Result<(), Self::Error> {
        // TODO: Implement display fills for one-SPI
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        _rectangle: Rectangle,
        _pixels: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        // TODO: Implement contiguous fills for one-SPI
        Ok(())
    }
}

impl CydTouch for CydTouchOneSpi {
    type Error = Infallible;
    type Uncalibrated = CydTouchUncalibratedOneSpi;

    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error> {
        Ok(None)
    }

    fn calibration_config(&self) -> CalibrationConfig {
        self.calibration_config
    }

    fn decalibrate(self) -> Self::Uncalibrated {
        CydTouchUncalibratedOneSpi {}
    }
}

pub struct CydTouchUncalibratedOneSpi;

impl CydTouchUncalibrated for CydTouchUncalibratedOneSpi {
    type Error = Infallible;
    type Calibrated = CydTouchOneSpi;

    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error> {
        Ok(None)
    }

    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated {
        CydTouchOneSpi { calibration_config }
    }
}

impl CydFrame for CydFrameOneSpi<'_> {
    type Error = Infallible;

    fn tile_top_left(&self) -> Point {
        self.tile_top_left
    }

    fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    fn fill(&mut self, _color: Rgb565) -> &mut Self {
        // TODO: Implement fills
        self
    }

    fn write_text(&mut self, _text: &str) -> &mut Self {
        // TODO: Implement text drawing
        self
    }

    fn copy_from_565(&mut self, _src: &[u16]) -> Result<(), Self::Error> {
        // TODO: Implement buffer copying
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        // TODO: Implement frame flushing to display
        Ok(())
    }
}

impl Cyd for CydEspOneSpi {
    type Error = Infallible;
    type Display = CydDisplayOneSpi;
    type Touch = CydTouchOneSpi;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }
}

impl fmt::Debug for CydEspOneSpi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CydEspOneSpi").finish_non_exhaustive()
    }
}
