//! Choose this bundle when the board exposes only one SPI peripheral for both
//! display and touch. It reduces wiring and peripheral use, at the cost of
//! arbitration and switching bus configurations between transactions.
//!
//! CYD bundle for one-SPI shared-bus designs where display and touch share a single SPI peripheral.
//!
//! This module provides [`CydEspOneSpi`], which arbitrates a single physical SPI bus between
//! the ILI9341 display and the XPT2046 touch controller using an
//! `embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig` per peripheral (each
//! with its own chip-select pin and its own fixed touch-bus clock). It reuses
//! the same display/touch drivers as the two-SPI [`super::CydEsp`], while the
//! uncalibrated touch implementation remains private to this crate
//! — so the only new code here is building the shared bus itself.

use core::{cell::RefCell, fmt};

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use esp_hal::{
    gpio::{Output, OutputConfig},
    spi,
};
use static_cell::StaticCell;

use device_envoy_core::button::Button;
use device_envoy_core::cyd::{Cyd, backend};

use super::{
    CydDisplayEsp, CydStaticEsp, CydTouchEsp, CydTouchUncalibratedEsp, Error, Orientation,
    TOUCH_SPI_HZ, buffer::PixelBuffer,
};
use crate::flash_block::FlashBlockEsp;

type SharedSpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
type SharedSpiMutex = Mutex<NoopRawMutex, RefCell<SharedSpiBus>>;
/// Both the display and touch device share this same concrete type — each instance just
/// carries its own `spi::master::Config` (clock speed), applied to the shared bus by
/// `SpiDeviceWithConfig` before every transaction it makes.
type SharedSpiDevice = SpiDeviceWithConfig<'static, NoopRawMutex, SharedSpiBus, Output<'static>>;

/// An ESP32 CYD device containing a display and calibrated touch input on one
/// shared SPI bus.
///
/// [`CydEspOneSpi::new_static`] creates the pixel buffer storage passed to
/// [`CydEspOneSpi::new`], which constructs the hardware and loads or performs
/// touch calibration. See the [`cyd`](super) module example for normal drawing
/// and touch input.
///
/// Display and touch retain independent chip-select pins and clock speeds while
/// sharing the physical bus. Use [`CydEsp`](super::CydEsp) when they use separate
/// SPI buses.
pub struct CydEspOneSpi {
    display: CydDisplayEsp<SharedSpiDevice>,
    touch: CydTouchEsp<SharedSpiDevice>,
}

impl CydEspOneSpi {
    /// Total pixel count of the CYD panel — fixed hardware, independent of orientation.
    ///
    /// See the compiled [`CydEspOneSpi::new`] constructor example.
    pub const SCREEN_PIXELS: usize = device_envoy_core::cyd::SCREEN_PIXELS;

    /// Create static storage for a CYD pixel buffer.
    ///
    /// Choose any `PIXEL_COUNT` from zero through
    /// [`CydEspOneSpi::SCREEN_PIXELS`].
    ///
    /// - `0` allocates no pixel buffer, so only
    ///   [immediate operations](super::CydDisplay::fill_rectangle) and
    ///   [contiguous streaming](super::CydDisplay::fill_contiguous) are
    ///   available.
    /// - A smaller buffer saves static RAM but limits the largest buffered
    ///   region.
    /// - For tiled drawing, size the buffer to
    ///   [`TileGrid::max_tile_pixel_count`](super::tiling::TileGrid::max_tile_pixel_count),
    ///   then pass the grid to
    ///   [`CydDisplay::for_each_tile`](super::CydDisplay::for_each_tile) to draw
    ///   the tiles. Only one tile is buffered at a time.
    /// - [`CydEspOneSpi::SCREEN_PIXELS`] allocates a full-screen buffer and is
    ///   usually the most convenient choice when enough RAM is available.
    ///
    /// Attempting to create a frame or tile larger than the allocated buffer
    /// panics. See [`CydStaticEsp`] for the complete sizing rules and the
    /// [`CydEspOneSpi::new`] constructor example.
    #[must_use]
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT> {
        CydStaticEsp::new()
    }

    /// Construct a ready-to-use one-SPI CYD.
    ///
    /// The display and touch controller share one SPI bus, with independent
    /// chip-select pins and clock speeds. The supplied flash block stores touch
    /// calibration, and `recalibration_button` requests interactive
    /// recalibration.
    ///
    /// Choosing the pixel buffer capacity is the most important construction
    /// decision: `statics` determines both static RAM use and the largest
    /// buffered region. See [`CydEspOneSpi::new_static`] for the sizing choices.
    ///
    /// Use [`CydEsp`](super::CydEsp) for boards where display and touch use
    /// separate SPI buses.
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// # use device_envoy_esp::{Result, button::{ButtonEsp, PressedTo}, cyd::{Cyd, CydEspOneSpi, CydStaticEsp, DEFAULT_DISPLAY_SPI_HZ, DEFAULT_FONT, Orientation}, flash_block::FlashBlockEsp};
    /// # use embedded_graphics::{pixelcolor::Rgb888, prelude::RgbColor};
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    /// # async fn construct(mut p: esp_hal::peripherals::Peripherals) -> Result<()> {
    /// #     let [mut calibration_flash] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    /// #     let mut recalibration_button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    ///     static CYD_STATIC: CydStaticEsp<{ CydEspOneSpi::SCREEN_PIXELS }> =
    ///         CydEspOneSpi::new_static();
    ///
    ///     let cyd = CydEspOneSpi::new(
    ///         &CYD_STATIC,
    ///
    ///         // Shared SPI and display pins:
    ///         p.SPI2,
    ///         p.GPIO1,
    ///         p.GPIO2,
    ///         p.GPIO3,
    ///         p.GPIO4,
    ///         p.GPIO5,
    ///         p.GPIO7,
    ///         p.GPIO8,
    ///         DEFAULT_DISPLAY_SPI_HZ,
    ///
    ///         // Touch pins:
    ///         p.GPIO12,
    ///         p.GPIO13,
    ///
    ///         // Presentation:
    ///         Orientation::Landscape,
    ///         Rgb888::BLACK,
    ///         Rgb888::WHITE,
    ///         &DEFAULT_FONT,
    ///
    ///         // Calibration storage and recalibration button:
    ///         &mut calibration_flash,
    ///         &mut recalibration_button,
    ///     )
    ///     .await?;
    /// #     assert_eq!(cyd.orientation(), Orientation::Landscape);
    /// #     Ok(())
    /// # }
    /// ```
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
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
        calibration_flash_block: &mut FlashBlockEsp,
        recalibration_button: &mut R,
    ) -> crate::Result<Self> {
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
            Orientation::Landscape,
            background_color,
            foreground_color,
            font,
            pixel_buffer,
        )?;
        let touch = CydTouchUncalibratedEsp::from_device(touch_spi_device, touch_irq_pin);

        let touch = backend::ensure_calibration(
            &mut display,
            touch,
            calibration_flash_block,
            recalibration_button,
            None,
            orientation,
        )
        .await
        .map_err(|error| match error {
            backend::Error::Device(cyd_error) => crate::Error::from(cyd_error),
            backend::Error::Flash(flash_error) => flash_error,
        })?;

        display.set_orientation(orientation)?;
        Ok(Self { display, touch })
    }
}

impl Cyd for CydEspOneSpi {
    type Error = Error;
    type Display = CydDisplayEsp<SharedSpiDevice>;
    type Touch = CydTouchEsp<SharedSpiDevice>;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }

    fn orientation(&self) -> Orientation {
        self.display.orientation
    }
}

impl fmt::Debug for CydEspOneSpi {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CydEspOneSpi")
            .field("orientation", &self.display.orientation)
            .finish_non_exhaustive()
    }
}
