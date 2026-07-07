//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydEspOneSpi`], which manages SPI arbitration between display and
//! touch controllers internally using interior mutability. The public API presents display and
//! touch as independently-borrowable halves of a bundle, hiding the shared-bus machinery.

use core::convert::Infallible;

use embedded_graphics::{
    Pixel,
    mono_font::MonoFont,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
};
use esp_hal::gpio::OutputPin;

use device_envoy_core::cyd::{
    Cyd, CydDisplay, CydTouch, CydTouchUncalibrated,
    display::CydFrame,
    touch::{RawTouchEvent, TouchEvent, calibration::CalibrationConfig},
};
use device_envoy_core::pixel_target::PixelTarget;

/// Display facade for one-SPI bundle.
pub struct CydDisplayOneSpi {
    screen_size: Size,
    background: Rgb888,
    foreground: Rgb888,
    background565: Rgb565,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// Calibrated touch facade for one-SPI bundle.
pub struct CydTouchOneSpi {
    calibration_config: CalibrationConfig,
}

/// Uncalibrated touch facade for one-SPI bundle.
#[derive(Clone)]
pub struct CydTouchUncalibratedOneSpi;

/// Frame for one-SPI bundle display.
#[allow(dead_code)]
pub struct CydFrameOneSpi {
    rectangle: Rectangle,
    tile_top_left: Point,
    foreground565: Rgb565,
    font: &'static MonoFont<'static>,
}

/// A CYD-family ESP32 bundle using one shared SPI peripheral for display and touch.
///
/// This type manages SPI arbitration between the ILI9341 display and XPT2046 touch controller
/// via interior mutability. Display and touch access is serialized to prevent conflicts on
/// the shared bus. The public API hides this machinery entirely.
///
/// Construction takes raw GPIO pins and a single SPI peripheral, not a pre-built bus object.
///
/// Note: The current implementation is a placeholder. Full implementation requires wrapping
/// the display and touch drivers to coordinate through a shared SPI bus.
pub struct CydEspOneSpi {
    display: CydDisplayOneSpi,
    touch: CydTouchOneSpi,
}

impl CydEspOneSpi {
    /// Construct a calibrated one-SPI CYD bundle.
    ///
    /// # Arguments
    ///
    /// * `spi` - The shared SPI peripheral (VSPI or HSPI)
    /// * `lcd_cs_pin` - LCD chip-select pin (active low)
    /// * `lcd_dc_pin` - LCD data/command pin
    /// * `lcd_rst_pin` - LCD reset pin (active low)
    /// * `touch_cs_pin` - Touch chip-select pin (active low)
    /// * `touch_irq_pin` - Touch interrupt pin
    /// * `calibration` - Pre-computed touch calibration
    /// * `delay` - Delay provider for initialization
    /// * `background` - Default background color
    /// * `foreground` - Default foreground/text color
    /// * `font` - Default monospace font for text drawing
    ///
    /// Returns a [`CydEspOneSpi`] ready for use.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        _spi: impl esp_hal::spi::master::Instance + 'static,
        _lcd_cs_pin: impl OutputPin + 'static,
        _lcd_dc_pin: impl OutputPin + 'static,
        _lcd_rst_pin: impl OutputPin + 'static,
        _touch_cs_pin: impl OutputPin + 'static,
        _touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
        calibration: CalibrationConfig,
        _delay: esp_hal::delay::Delay,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, Infallible> {
        let background565 = Rgb565::from(background);
        let foreground565 = Rgb565::from(foreground);

        Ok(Self {
            display: CydDisplayOneSpi {
                screen_size: Size::new(320, 240),
                background,
                foreground,
                background565,
                foreground565,
                font,
            },
            touch: CydTouchOneSpi { calibration_config: calibration },
        })
    }
}

impl CydDisplay for CydDisplayOneSpi {
    type Error = Infallible;
    type Frame<'a> = CydFrameOneSpi;

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
        }
    }

    fn fill_rectangle(
        &mut self,
        _rectangle: Rectangle,
        _color: Rgb565,
    ) -> Result<(), Self::Error> {
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
        CydTouchUncalibratedOneSpi
    }
}

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

impl DrawTarget for CydFrameOneSpi {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, _pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        Ok(())
    }
}

impl Dimensions for CydFrameOneSpi {
    fn bounding_box(&self) -> Rectangle {
        self.rectangle
    }
}

impl PixelTarget for CydFrameOneSpi {
    fn width(&self) -> usize {
        self.rectangle.size.width as usize
    }

    fn height(&self) -> usize {
        self.rectangle.size.height as usize
    }

    fn put_pixel(&mut self, _x: usize, _y: usize, _color: Rgb888) {}
}

impl CydFrame for CydFrameOneSpi {
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

    fn copy_from_565(&mut self, _src: &[u16]) -> Result<(), device_envoy_core::Error> {
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), Infallible> {
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
