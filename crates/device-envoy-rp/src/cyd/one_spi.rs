//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydRpOneSpi`], which arbitrates a single physical SPI bus between
//! the ST7789 display and the XPT2046 touch controller using an
//! `embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig` per peripheral (each
//! with its own chip-select pin *and* its own SPI clock speed — see [`super::DEFAULT_DISPLAY_SPI_HZ`] vs
//! [`TOUCH_SPI_HZ`]). It reuses the same display/touch drivers as the two-SPI [`super::CydRp`] —
//! see [`super::CydDisplayRp::new_from_device`] and [`super::CydTouchUncalibratedRp::from_device`]
//! — so the only new code here is building the shared bus itself.
//!
//! Unlike [`super::CydEspOneSpi`](../../device_envoy_esp/cyd/struct.CydEspOneSpi.html)'s bus, which
//! is type-erased over any ESP SPI peripheral, `embassy_rp::spi::Spi` carries its peripheral
//! (`SPI0`/`SPI1`) as a type parameter, so [`CydRpOneSpi`] and its static storage,
//! [`CydRpOneSpiStatic`], are generic over that peripheral instance `T`.

use core::cell::RefCell;

use device_envoy_core::button::Button;
use device_envoy_core::cyd::{Cyd, touch::calibration::ensure_calibration};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::spi::{
    self, Blocking, ClkPin, Config as SpiConfig, MisoPin, MosiPin, Phase, Polarity, Spi,
};
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use static_cell::StaticCell;

use super::{
    CydDisplayRp, CydTouchRp, CydTouchUncalibratedRp, Error, Orientation, PixelBuffer, TOUCH_SPI_HZ,
};
use crate::flash_block::FlashBlockRp;

type SharedSpiBus<T> = Spi<'static, T, Blocking>;
type SharedSpiMutex<T> = Mutex<NoopRawMutex, RefCell<SharedSpiBus<T>>>;
/// Both the display and touch device share this same concrete type — each instance just
/// carries its own `embassy_rp::spi::Config` (clock speed), applied to the shared bus by
/// [`SpiDeviceWithConfig`] before every transaction it makes.
type SharedSpiDevice<T> =
    SpiDeviceWithConfig<'static, NoopRawMutex, SharedSpiBus<T>, Output<'static>>;

/// A CYD-family RP bundle using one shared SPI peripheral for display and touch.
///
/// Display and touch each get their own [`SpiDeviceWithConfig`] over the same underlying bus,
/// with independent chip-select pins *and* independent clock speeds: [`SpiDeviceWithConfig`]
/// re-applies its device's `embassy_rp::spi::Config` to the shared bus immediately before each of
/// its transactions, so the physical SPI clock switches between [`super::DEFAULT_DISPLAY_SPI_HZ`] and
/// [`TOUCH_SPI_HZ`] as display and touch take turns using the bus. Because the two halves share
/// state through that bus, this type keeps shared-bus ownership atomic inside the complete
/// [`Cyd`] bundle.
///
/// `T` is the SPI peripheral instance (`SPI0` or `SPI1`) the shared bus runs on; see
/// [`CydRpOneSpiStatic`] for why the static storage must name the same `T`.
pub struct CydRpOneSpi<T: spi::Instance + 'static> {
    display: CydDisplayRp<SharedSpiDevice<T>>,
    touch: CydTouchRp<SharedSpiDevice<T>>,
}

/// Static storage for a [`CydRpOneSpi`]-owned draw buffer and shared SPI bus.
///
/// Unlike [`super::CydStaticRp`], this bundles the shared-bus mutex alongside the pixel buffer:
/// `embassy_rp::spi::Spi<'static, T, Blocking>` carries its peripheral instance `T` as a type
/// parameter, and a `static` item cannot reference a generic parameter of the function that
/// creates it — so the caller must declare this storage (naming a concrete `T`) at module scope,
/// same as any other multi-instance device in this crate.
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::cyd::{CydRpOneSpi, CydRpOneSpiStatic};
/// use embassy_rp::peripherals::SPI0;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// static CYD_STATIC: CydRpOneSpiStatic<SPI0, { CydRpOneSpi::<SPI0>::SCREEN_PIXELS }> =
///     CydRpOneSpi::new_static();
/// ```
pub struct CydRpOneSpiStatic<T: spi::Instance + 'static, const PIXEL_COUNT: usize> {
    pixel_buffer: StaticCell<PixelBuffer<PIXEL_COUNT>>,
    shared_spi: StaticCell<SharedSpiMutex<T>>,
}

impl<T: spi::Instance + 'static, const PIXEL_COUNT: usize> CydRpOneSpiStatic<T, PIXEL_COUNT> {
    /// Internal constructor. Apps create storage via [`CydRpOneSpi::new_static`] so all
    /// construction goes through the `CydRpOneSpi` device abstraction.
    pub(crate) const fn new() -> Self {
        Self {
            pixel_buffer: StaticCell::new(),
            shared_spi: StaticCell::new(),
        }
    }
}

impl<T: spi::Instance + 'static> CydRpOneSpi<T> {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    pub const SCREEN_PIXELS: usize = device_envoy_core::cyd::SCREEN_PIXELS;

    /// Create [`CydRpOneSpiStatic`] storage for a `PIXEL_COUNT`-sized draw buffer and the shared
    /// SPI bus.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydRpOneSpiStatic<T, PIXEL_COUNT> {
        CydRpOneSpiStatic::new()
    }

    /// Construct a calibrated one-SPI CYD bundle using the saved-or-interactive calibration flow.
    ///
    /// Mirrors [`super::CydRp::new`]'s calibration handling exactly (same
    /// [`ensure_calibration`] flow, same flash-backed load/save behavior) — the only
    /// difference from the two-SPI bundle is that display and touch share one physical bus.
    ///
    /// # Arguments
    ///
    /// * `statics` - Static storage for the shared SPI bus and the display's draw buffer
    /// * `spi` - The shared SPI peripheral
    /// * `sck_pin` / `mosi_pin` / `miso_pin` - Shared bus pins for both display and touch
    /// * `lcd_cs_pin` - LCD chip-select pin (active low)
    /// * `lcd_dc_pin` - LCD data/command pin
    /// * `lcd_rst_pin` - LCD reset pin (active low)
    /// * `lcd_backlight_pin` - LCD backlight enable pin
    /// * `display_spi_hz` - SPI clock used for display transactions
    /// * `touch_cs_pin` - Touch chip-select pin (active low)
    /// * `touch_irq_pin` - Touch interrupt pin
    /// * `orientation` - Screen orientation
    /// * `background` - Default background color
    /// * `foreground` - Default foreground/text color
    /// * `font` - Default monospace font for text drawing
    /// * `calibration_flash_block` - Flash block used to load/save the touch calibration
    /// * `recalibration_button` - Button that restarts the interactive calibration flow
    ///
    /// Returns a ready-to-use [`CydRpOneSpi`].
    #[expect(clippy::too_many_arguments, reason = "mirrors CydEspOneSpi::new")]
    pub async fn new<
        const PIXEL_COUNT: usize,
        Sck,
        Mosi,
        Miso,
        LcdCs,
        Dc,
        Rst,
        Backlight,
        TouchCs,
        TouchIrq,
        R: Button,
    >(
        statics: &'static CydRpOneSpiStatic<T, PIXEL_COUNT>,
        spi: Peri<'static, T>,
        sck_pin: Peri<'static, Sck>,
        mosi_pin: Peri<'static, Mosi>,
        miso_pin: Peri<'static, Miso>,
        lcd_cs_pin: Peri<'static, LcdCs>,
        lcd_dc_pin: Peri<'static, Dc>,
        lcd_rst_pin: Peri<'static, Rst>,
        lcd_backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        touch_cs_pin: Peri<'static, TouchCs>,
        touch_irq_pin: Peri<'static, TouchIrq>,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
        calibration_flash_block: &mut FlashBlockRp,
        recalibration_button: &mut R,
    ) -> crate::Result<Self>
    where
        Sck: Pin + ClkPin<T>,
        Mosi: Pin + MosiPin<T>,
        Miso: Pin + MisoPin<T>,
        LcdCs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
        TouchCs: Pin,
        TouchIrq: Pin,
    {
        // The bus's own construction-time config barely matters: every transaction through
        // either `SharedSpiDevice` below re-applies its own config first (see
        // `SpiDeviceWithConfig`), so this initial value is immediately overwritten before any
        // real transfer happens. `TOUCH_SPI_HZ` is used here only as a conservative starting
        // point.
        let spi_config = {
            let mut spi_config = SpiConfig::default();
            spi_config.frequency = TOUCH_SPI_HZ;
            spi_config.polarity = Polarity::IdleLow;
            spi_config.phase = Phase::CaptureOnFirstTransition;
            spi_config
        };
        // Touch reads response bytes over this bus, so — unlike the two-SPI `CydDisplayRp`'s
        // TX-only display bus — this shared bus must be full-duplex.
        let spi = Spi::new_blocking(spi, sck_pin, mosi_pin, miso_pin, spi_config);

        let shared_spi: &'static SharedSpiMutex<T> =
            statics.shared_spi.init(Mutex::new(RefCell::new(spi)));

        let lcd_cs = Output::new(lcd_cs_pin, Level::High);
        let touch_cs = Output::new(touch_cs_pin, Level::High);

        // The ST7789 display tolerates a much faster clock than the XPT2046 touch
        // controller; each device carries its own `Config`, applied to the bus immediately
        // before its own transactions (mirrors the ESP one-SPI bundle's measured rationale).
        let lcd_spi_config = {
            let mut lcd_spi_config = SpiConfig::default();
            lcd_spi_config.frequency = display_spi_hz;
            lcd_spi_config.polarity = Polarity::IdleLow;
            lcd_spi_config.phase = Phase::CaptureOnFirstTransition;
            lcd_spi_config
        };
        let touch_spi_config = {
            let mut touch_spi_config = SpiConfig::default();
            touch_spi_config.frequency = TOUCH_SPI_HZ;
            touch_spi_config.polarity = Polarity::IdleLow;
            touch_spi_config.phase = Phase::CaptureOnFirstTransition;
            touch_spi_config
        };

        let lcd_spi_device = SpiDeviceWithConfig::new(shared_spi, lcd_cs, lcd_spi_config);
        let touch_spi_device = SpiDeviceWithConfig::new(shared_spi, touch_cs, touch_spi_config);

        let pixel_buffer = PixelBuffer::init_static(&statics.pixel_buffer);
        let mut display = CydDisplayRp::new_from_device(
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
        let touch = CydTouchUncalibratedRp::from_device(touch_spi_device, touch_irq_pin);

        let (touch, _) = ensure_calibration(
            &mut display,
            touch,
            calibration_flash_block,
            recalibration_button,
            None,
        )
        .await
        .map_err(|error| match error.kind {
            device_envoy_core::cyd::touch::calibration::ErrorKind::Device(cyd_error) => {
                crate::Error::from(cyd_error)
            }
            device_envoy_core::cyd::touch::calibration::ErrorKind::Flash(flash_error) => {
                flash_error
            }
        })?;

        Ok(Self { display, touch })
    }
}

impl<T: spi::Instance + 'static> Cyd for CydRpOneSpi<T> {
    type Error = Error;
    type Display = CydDisplayRp<SharedSpiDevice<T>>;
    type Touch = CydTouchRp<SharedSpiDevice<T>>;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }

    fn orientation(&self) -> Orientation {
        self.display.orientation
    }
}

impl<T: spi::Instance + 'static> core::fmt::Debug for CydRpOneSpi<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CydRpOneSpi")
            .finish_non_exhaustive()
    }
}
