use core::convert::Infallible;

use device_envoy_core::UnwrapInfallible;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::MonoFont,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{Point, Size},
    primitives::Rectangle,
};
use embedded_hal::spi::SpiDevice;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    delay::Delay,
    gpio::{
        Level, Output, OutputConfig, OutputPin,
        interconnect::{PeripheralInput, PeripheralOutput},
    },
    spi,
};
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ILI9341Rgb565,
    options::{ColorOrder, Orientation as MipiOrientation, Rotation},
};
use static_cell::StaticCell;

use super::{CydFrameEsp, Orientation, buffer::DynPixelBuffer};

// 60 MHz is the current tested default for CYD display traffic on ESP: 80 MHz
// was faster in measurement but produced visible corruption on the tested
// panel.
/// Default SPI clock for CYD display traffic on ESP boards.
///
/// See the [`CydDisplayEsp::new`](super::CydDisplayEsp::new) constructor example.
pub const DEFAULT_DISPLAY_SPI_HZ: u32 = 60_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

type CydDisplaySpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
/// The SPI device type used when the display owns an exclusive SPI peripheral.
pub(crate) type CydDisplaySpiDevice = ExclusiveDevice<CydDisplaySpiBus, Output<'static>, NoDelay>;
type CydDisplayInterface<D> = SpiInterface<'static, D, Output<'static>>;
type CydDisplayDevice<D> =
    mipidsi::Display<CydDisplayInterface<D>, ILI9341Rgb565, DisplayResetOutput>;

/// A display-reset argument accepted by the CYD constructors.
///
/// Ordinary ESP GPIO output pins implement this trait automatically. Pass
/// [`NoDisplayReset`] when the display reset line shares the board reset/EN net.
pub trait DisplayResetPin: reset_pin::Sealed {}

impl<Pin: OutputPin + 'static> DisplayResetPin for Pin {}

/// Indicates that a CYD display has no dedicated reset GPIO.
///
/// Pass this to a CYD constructor when the display reset line shares the board
/// reset/EN net, as it does on the classic ESP32-2432S028R.
pub struct NoDisplayReset;

impl DisplayResetPin for NoDisplayReset {}

pub enum DisplayResetOutput {
    Pin(Output<'static>),
    Board,
}

impl embedded_hal::digital::ErrorType for DisplayResetOutput {
    type Error = Infallible;
}

impl embedded_hal::digital::OutputPin for DisplayResetOutput {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        if let Self::Pin(pin) = self {
            pin.set_low();
        }
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        if let Self::Pin(pin) = self {
            pin.set_high();
        }
        Ok(())
    }
}

mod reset_pin {
    use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};

    use super::{DisplayResetOutput, NoDisplayReset};

    pub trait Sealed {
        fn into_output(self) -> DisplayResetOutput;
    }

    impl<Pin: OutputPin + 'static> Sealed for Pin {
        fn into_output(self) -> DisplayResetOutput {
            DisplayResetOutput::Pin(Output::new(self, Level::High, OutputConfig::default()))
        }
    }

    impl Sealed for NoDisplayReset {
        fn into_output(self) -> DisplayResetOutput {
            DisplayResetOutput::Board
        }
    }
}

fn orientation_to_mipi(orientation: Orientation) -> MipiOrientation {
    match orientation {
        Orientation::Landscape => MipiOrientation::new()
            .rotate(Rotation::Deg90)
            .flip_horizontal()
            .rotate(Rotation::Deg180),
        Orientation::Portrait => MipiOrientation::new()
            .rotate(Rotation::Deg180)
            .flip_horizontal(),
        Orientation::LandscapeInverted => MipiOrientation::new()
            .rotate(Rotation::Deg90)
            .flip_horizontal(),
        Orientation::PortraitInverted => MipiOrientation::new().flip_horizontal(),
    }
}

/// An ILI9341 display driven over `D`, an `embedded-hal` SPI device.
///
/// `D` defaults to [`CydDisplaySpiDevice`], an exclusively-owned SPI
/// peripheral. Shared-bus backends (see `one_spi`) instead construct this
/// with an `embedded_hal_bus::spi::RefCellDevice` via [`CydDisplayEsp::new_from_device`].
pub(crate) struct CydDisplayEsp<D: SpiDevice<u8> = CydDisplaySpiDevice> {
    display: CydDisplayDevice<D>,
    screen_size: Size,
}

impl<D: SpiDevice<u8>> CydDisplayEsp<D> {
    pub(crate) fn set_orientation(&mut self, orientation: Orientation) -> Result<(), super::Error> {
        self.display
            .set_orientation(orientation_to_mipi(orientation))
            .map_err(|_| super::Error::SetOrientation)?;
        self.screen_size = orientation.size();
        Ok(())
    }

    /// Oriented screen size stored at init time.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.screen_size
    }

    #[must_use]
    fn screen_rectangle(&self) -> Rectangle {
        Rectangle::new(Point::new(0, 0), self.screen_size)
    }

    /// Construct a display driver from an already-built SPI device.
    ///
    /// Used by shared-bus backends that build their own `SpiDevice` (for
    /// example an `embedded_hal_bus::spi::RefCellDevice` wrapping a bus
    /// shared with touch) instead of owning an exclusive SPI peripheral.
    pub(crate) fn new_from_device(
        spi_device: D,
        dc_pin: impl OutputPin + 'static,
        rst_pin: impl DisplayResetPin,
        backlight_pin: impl OutputPin + 'static,
        orientation: Orientation,
    ) -> Result<CydDisplayEsp<D>, super::Error> {
        let dc = Output::new(dc_pin, Level::Low, OutputConfig::default());
        let rst = reset_pin::Sealed::into_output(rst_pin);
        let mut backlight = Output::new(backlight_pin, Level::High, OutputConfig::default());

        static SPI_BUFFER: StaticCell<[u8; DISPLAY_SPI_BUFFER_LEN]> = StaticCell::new();
        let spi_buffer = SPI_BUFFER.init([0u8; DISPLAY_SPI_BUFFER_LEN]);
        let interface = SpiInterface::new(spi_device, dc, spi_buffer);
        let mut delay = Delay::new();

        let screen_size = orientation.size();
        let display = Builder::new(ILI9341Rgb565, interface)
            .reset_pin(rst)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .orientation(orientation_to_mipi(orientation))
            .init(&mut delay)
            // `InitError` is parameterized by the private SPI-device type. The
            // public CYD error keeps the operation context while the concrete
            // device error remains behind this platform abstraction.
            .map_err(|_| super::Error::InitDisplay)?;

        backlight.set_high();

        Ok(CydDisplayEsp {
            display,
            screen_size,
        })
    }

    pub(crate) fn flush_buffer(
        &mut self,
        width: usize,
        height: usize,
        pixels: &[u16],
        top_left: Point,
    ) -> Result<(), super::Error> {
        let rectangle = Rectangle::new(top_left, Size::new(width as u32, height as u32));
        self.display
            .fill_contiguous(
                &rectangle,
                pixels
                    .iter()
                    .copied()
                    .map(|pixel| Rgb565::from(RawU16::new(pixel))),
            )
            // The display error is parameterized by the private SPI-device
            // type; retain the public operation context at this boundary.
            .map_err(|_| super::Error::FlushFrameBuffer)
    }

    pub(crate) fn make_frame<'a>(
        &'a mut self,
        pixel_buffer: &'a mut dyn DynPixelBuffer,
        rectangle: Rectangle,
        background565: Rgb565,
        foreground565: Rgb565,
        font: &'static MonoFont<'static>,
    ) -> CydFrameEsp<'a, D> {
        let size = rectangle.size;
        let mut view = pixel_buffer.view_mut(size.width as usize, size.height as usize);
        // Every new frame starts cleared to the device background so callers
        // never have to clear it themselves.
        view.fill(background565);
        CydFrameEsp {
            display: self,
            view,
            rectangle,
            background565,
            foreground565,
            font,
        }
    }

    pub(crate) fn fill(&mut self, color: Rgb565) -> Result<(), super::Error> {
        self.fill_rectangle(self.screen_rectangle(), color)
    }

    pub(crate) fn fill_rectangle(
        &mut self,
        rectangle: Rectangle,
        color: Rgb565,
    ) -> Result<(), super::Error> {
        self.display
            .fill_solid(&rectangle, color)
            .map_err(|_| super::Error::FlushFrameBuffer)
    }

    pub(crate) fn fill_contiguous<I>(
        &mut self,
        rectangle: Rectangle,
        pixels: I,
    ) -> Result<(), super::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return Ok(());
        }
        self.display
            .fill_contiguous(&rectangle, pixels)
            .map_err(|_| super::Error::FlushFrameBuffer)
    }
}

impl CydDisplayEsp<CydDisplaySpiDevice> {
    pub(crate) fn new(
        spi: impl spi::master::Instance + 'static,
        sck_pin: impl PeripheralOutput<'static>,
        mosi_pin: impl PeripheralOutput<'static>,
        miso_pin: impl PeripheralInput<'static>,
        cs_pin: impl OutputPin + 'static,
        dc_pin: impl OutputPin + 'static,
        rst_pin: impl DisplayResetPin,
        backlight_pin: impl OutputPin + 'static,
        display_spi_hz: u32,
        orientation: Orientation,
    ) -> Result<CydDisplayEsp<CydDisplaySpiDevice>, super::Error> {
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(display_spi_hz))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)
            .map_err(super::Error::ConfigureDisplaySpi)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        let cs = Output::new(cs_pin, Level::High, OutputConfig::default());

        let spi_device =
            ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs).unwrap_infallible();

        Self::new_from_device(spi_device, dc_pin, rst_pin, backlight_pin, orientation)
    }
}
