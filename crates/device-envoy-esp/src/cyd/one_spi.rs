//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydEspOneSpi`], which arbitrates a single physical SPI bus between
//! the ILI9341 display and the XPT2046 touch controller using an
//! `embedded_hal_bus::spi::RefCellDevice` per peripheral (each with its own chip-select pin).
//! It reuses the same display/touch drivers as the two-SPI [`super::CydEsp`] — see
//! [`super::CydDisplayEsp::new_from_device`] and [`super::CydTouchUncalibratedEsp::from_device`] —
//! so the only new code here is building the shared bus itself.

use core::cell::RefCell;

use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use embedded_hal_bus::spi::{NoDelay, RefCellDevice};
use esp_hal::{
    gpio::{Output, OutputConfig},
    spi,
};
use static_cell::StaticCell;

use device_envoy_core::button::Button;
use device_envoy_core::cyd::{
    Cyd,
    touch::calibration::{EnsureCalibrationOutcome, ensure_calibration},
};

use super::{
    CydDisplayEsp, CydError, CydStaticEsp, CydTouchEsp, CydTouchUncalibratedEsp, Orientation,
    TOUCH_SPI_HZ, buffer::PixelBuffer,
};
use crate::flash_block::FlashBlockEsp;

type SharedSpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
type SharedSpiDevice = RefCellDevice<'static, SharedSpiBus, Output<'static>, NoDelay>;

/// A CYD-family ESP32 bundle using one shared SPI peripheral for display and touch.
///
/// Display and touch each get their own `embedded_hal_bus::spi::RefCellDevice` over the same
/// underlying bus, with independent chip-select pins, so accesses are serialized through the
/// `RefCell` rather than requiring two physical SPI peripherals. Because the two halves share
/// state through that bus, this type implements [`Cyd`] but not
/// [`CydParts`](device_envoy_core::cyd::CydParts) — see that trait's documentation for why
/// shared-bus backends cannot safely split into independently-owned parts.
///
/// The shared bus runs at [`TOUCH_SPI_HZ`] (2.5 MHz), the XPT2046 touch controller's ceiling,
/// since both peripherals share one clock configuration. This is slower than the two-SPI
/// design's 60 MHz display bus — an accepted trade-off for boards with only one SPI peripheral.
pub struct CydEspOneSpi {
    display: CydDisplayEsp<SharedSpiDevice>,
    touch: CydTouchEsp<SharedSpiDevice>,
}

impl CydEspOneSpi {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    pub const SCREEN_PIXELS: usize = device_envoy_core::cyd::SCREEN_PIXELS;

    /// Create [`CydStaticEsp`] storage for a `PIXEL_COUNT`-sized draw buffer.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT> {
        CydStaticEsp::new()
    }

    /// Construct a calibrated one-SPI CYD bundle using the saved-or-interactive calibration flow.
    ///
    /// Mirrors [`super::CydEsp::new`]'s calibration handling exactly (same
    /// [`ensure_calibration`] flow, same flash-backed load/save behavior) — the only
    /// difference from the two-SPI bundle is that display and touch share one physical bus.
    ///
    /// # Arguments
    ///
    /// * `statics` - Static storage for the display's draw buffer
    /// * `spi` - The shared SPI peripheral
    /// * `sck_pin` / `mosi_pin` / `miso_pin` - Shared bus pins for both display and touch
    /// * `lcd_cs_pin` - LCD chip-select pin (active low)
    /// * `lcd_dc_pin` - LCD data/command pin
    /// * `lcd_rst_pin` - LCD reset pin (active low)
    /// * `lcd_backlight_pin` - LCD backlight enable pin
    /// * `touch_cs_pin` - Touch chip-select pin (active low)
    /// * `touch_irq_pin` - Touch interrupt pin
    /// * `orientation` - Screen orientation
    /// * `background` - Default background color
    /// * `foreground` - Default foreground/text color
    /// * `font` - Default monospace font for text drawing
    /// * `calibration_flash_block` - Flash block used to load/save the touch calibration
    /// * `recalibration_button` - Button that restarts the interactive calibration flow
    /// * `confirmed_message` - Message shown after a fresh calibration is saved
    ///
    /// Returns a [`CydEspOneSpi`] ready for use, along with how calibration was obtained.
    #[allow(clippy::too_many_arguments)]
    pub async fn new<const PIXEL_COUNT: usize, R: Button>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        spi: impl spi::master::Instance + 'static,
        sck_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        miso_pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        lcd_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        lcd_dc_pin: impl esp_hal::gpio::OutputPin + 'static,
        lcd_rst_pin: impl esp_hal::gpio::OutputPin + 'static,
        lcd_backlight_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_cs_pin: impl esp_hal::gpio::OutputPin + 'static,
        touch_irq_pin: impl esp_hal::gpio::InputPin + 'static,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        calibration_flash_block: &mut FlashBlockEsp,
        recalibration_button: &mut R,
        confirmed_message: Option<&str>,
    ) -> crate::Result<(Self, EnsureCalibrationOutcome)> {
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(TOUCH_SPI_HZ))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        static SHARED_SPI: StaticCell<RefCell<SharedSpiBus>> = StaticCell::new();
        let shared_spi: &'static RefCell<SharedSpiBus> = SHARED_SPI.init(RefCell::new(spi));

        let lcd_cs = Output::new(
            lcd_cs_pin,
            esp_hal::gpio::Level::High,
            OutputConfig::default(),
        );
        let touch_cs = Output::new(
            touch_cs_pin,
            esp_hal::gpio::Level::High,
            OutputConfig::default(),
        );

        // `Output`'s digital error type is `Infallible` (see esp-hal's
        // `embedded_hal_impls`), so `RefCellDevice::new_no_delay` can't actually fail here.
        let lcd_spi_device = RefCellDevice::new_no_delay(shared_spi, lcd_cs)
            .unwrap_or_else(|infallible| match infallible {});
        let touch_spi_device = RefCellDevice::new_no_delay(shared_spi, touch_cs)
            .unwrap_or_else(|infallible| match infallible {});

        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);
        let mut display = CydDisplayEsp::new_from_device(
            lcd_spi_device,
            lcd_dc_pin,
            lcd_rst_pin,
            lcd_backlight_pin,
            orientation,
            background,
            foreground,
            font,
            pixel_buffer,
        )?;
        let touch = CydTouchUncalibratedEsp::from_device(touch_spi_device, touch_irq_pin);

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

impl Cyd for CydEspOneSpi {
    type Error = CydError;
    type Display = CydDisplayEsp<SharedSpiDevice>;
    type Touch = CydTouchEsp<SharedSpiDevice>;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }
}
