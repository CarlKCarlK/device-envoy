//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydEspOneSpi`], which arbitrates a single physical SPI bus between
//! the ILI9341 display and the XPT2046 touch controller using an
//! `embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig` per peripheral (each
//! with its own chip-select pin *and* its own SPI clock speed — see [`super::DEFAULT_DISPLAY_SPI_HZ`] vs
//! [`TOUCH_SPI_HZ`]). It reuses the same display/touch drivers as the two-SPI [`super::CydEsp`] —
//! see [`super::CydDisplayEsp::new_from_device`] and [`super::CydTouchUncalibratedEsp::from_device`]
//! — so the only new code here is building the shared bus itself.

use core::cell::RefCell;

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use esp_hal::{
    gpio::{Output, OutputConfig},
    spi,
};
use static_cell::StaticCell;

use device_envoy_core::button::Button;
use device_envoy_core::cyd::{
    Cyd, CydUncalibrated,
    touch::calibration::{EnsureCalibrationOutcome, ensure_calibration},
};

use super::{
    CydDisplayEsp, CydError, CydStaticEsp, CydTouchEsp, CydTouchUncalibratedEsp, Orientation,
    TOUCH_SPI_HZ, buffer::PixelBuffer,
};
use crate::flash_block::FlashBlockEsp;

type SharedSpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
type SharedSpiMutex = Mutex<NoopRawMutex, RefCell<SharedSpiBus>>;
/// Both the display and touch device share this same concrete type — each instance just
/// carries its own [`spi::master::Config`] (clock speed), applied to the shared bus by
/// [`SpiDeviceWithConfig`] before every transaction it makes.
type SharedSpiDevice = SpiDeviceWithConfig<'static, NoopRawMutex, SharedSpiBus, Output<'static>>;

/// A CYD-family ESP32 bundle using one shared SPI peripheral for display and touch.
///
/// Display and touch each get their own [`SpiDeviceWithConfig`] over the same underlying bus,
/// with independent chip-select pins *and* independent clock speeds: [`SpiDeviceWithConfig`]
/// re-applies its device's [`spi::master::Config`] to the shared bus immediately before each of
/// its transactions, so the physical SPI clock switches between [`super::DEFAULT_DISPLAY_SPI_HZ`] and
/// [`TOUCH_SPI_HZ`] as display and touch take turns using the bus. Because the two halves share
/// state through that bus, this type implements [`Cyd`] but not
/// [`CydParts`](device_envoy_core::cyd::CydParts) — see that trait's documentation for why
/// shared-bus backends cannot safely split into independently-owned parts.
pub struct CydEspOneSpi {
    display: CydDisplayEsp<SharedSpiDevice>,
    touch: CydTouchEsp<SharedSpiDevice>,
}

/// An uncalibrated one-SPI CYD bundle.
///
/// The display and raw touch handles stay together because they share the same
/// physical SPI bus. This type intentionally does not implement `CydParts`.
pub struct CydEspOneSpiUncalibrated {
    display: CydDisplayEsp<SharedSpiDevice>,
    touch: CydTouchUncalibratedEsp<SharedSpiDevice>,
}

impl CydUncalibrated for CydEspOneSpiUncalibrated {
    type Calibrated = CydEspOneSpi;
    type Error = CydError;

    fn into_calibrated<F, B>(
        self,
        _calibration_flash_block: &mut F,
        _recalibration_button: &mut B,
    ) -> impl core::future::Future<Output = Result<Self::Calibrated, Self::Error>>
    where
        F: device_envoy_core::flash_block::FlashBlock,
        B: Button,
        Self::Error: From<F::Error>,
    {
        async { todo!("todo0000 fix this up") }
    }
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
        display_spi_hz: u32,
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
        // The bus's own construction-time config barely matters: every transaction through
        // either `SharedSpiDevice` below re-applies its own config first (see
        // `SpiDeviceWithConfig`), so this initial value is immediately overwritten before any
        // real transfer happens. `TOUCH_SPI_HZ` is used here only as a conservative starting
        // point.
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(TOUCH_SPI_HZ))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        static SHARED_SPI: StaticCell<SharedSpiMutex> = StaticCell::new();
        let shared_spi: &'static SharedSpiMutex = SHARED_SPI.init(Mutex::new(RefCell::new(spi)));

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

        // The ILI9341 display tolerates a much faster clock than the XPT2046 touch
        // controller; running the shared bus at the touch controller's ceiling for both (as
        // this bundle used to) capped every full-frame flush at roughly 2 fps. Each device now
        // carries its own `Config`, applied to the bus immediately before its own transactions.
        //
        // Measured on real ESP32-C6 hardware: a requested 60 MHz gave ~59ms/16.8fps full-frame
        // flushes vs. an explicit 20 MHz's ~90ms/11fps — confirming per-device config really
        // does take effect and 60 MHz is the better choice. Both numbers run well above their
        // pure-bit-rate predictions (~20ms and ~61ms respectively); the gap is fixed per-frame
        // overhead (ILI9341 addressing commands, CS/DC toggling, driver-side pixel iteration)
        // that doesn't scale with SPI clock, not a bug in this config-switching approach.
        let lcd_spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(display_spi_hz))
            .with_mode(spi::Mode::_0);
        let touch_spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(TOUCH_SPI_HZ))
            .with_mode(spi::Mode::_0);

        let lcd_spi_device = SpiDeviceWithConfig::new(shared_spi, lcd_cs, lcd_spi_config);
        let touch_spi_device = SpiDeviceWithConfig::new(shared_spi, touch_cs, touch_spi_config);

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
