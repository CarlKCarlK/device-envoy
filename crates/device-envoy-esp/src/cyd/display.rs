use device_envoy_core::cyd::display::RectanglePixels;
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
pub const DEFAULT_DISPLAY_SPI_HZ: u32 = 60_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

type CydDisplaySpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
/// The SPI device type used when the display owns an exclusive SPI peripheral.
pub(crate) type CydDisplaySpiDevice = ExclusiveDevice<CydDisplaySpiBus, Output<'static>, NoDelay>;
type CydDisplayInterface<D> = SpiInterface<'static, D, Output<'static>>;
type CydDisplayDevice<D> = mipidsi::Display<CydDisplayInterface<D>, ILI9341Rgb565, Output<'static>>;

/// Error initializing the display over SPI.
#[derive(Clone, Copy, Debug)]
pub enum CydDisplayEspInitError {
    /// Configuring the display SPI peripheral failed.
    ConfigureDisplaySpi,
    /// Wrapping the display SPI bus with its CS pin failed.
    CreateDisplaySpiDevice,
    /// The mipidsi display driver failed to initialize the panel.
    InitDisplay,
}

/// Error flushing pixels to the display.
#[derive(Clone, Copy, Debug)]
pub enum CydDisplayEspFlushError {
    /// Writing the frame buffer to the panel over SPI failed.
    FlushFrameBuffer,
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
        rst_pin: impl OutputPin + 'static,
        backlight_pin: impl OutputPin + 'static,
        orientation: Orientation,
    ) -> Result<CydDisplayEsp<D>, CydDisplayEspInitError> {
        let dc = Output::new(dc_pin, Level::Low, OutputConfig::default());
        let rst = Output::new(rst_pin, Level::High, OutputConfig::default());
        let mut backlight = Output::new(backlight_pin, Level::High, OutputConfig::default());

        static SPI_BUFFER: StaticCell<[u8; DISPLAY_SPI_BUFFER_LEN]> = StaticCell::new();
        let spi_buffer = SPI_BUFFER.init([0u8; DISPLAY_SPI_BUFFER_LEN]);
        let interface = SpiInterface::new(spi_device, dc, spi_buffer);
        let mut delay = Delay::new();

        let screen_size = orientation.size();
        let display_orientation = match orientation {
            Orientation::Landscape => MipiOrientation::new()
                .rotate(Rotation::Deg90)
                .flip_horizontal()
                .rotate(Rotation::Deg180),
            Orientation::Portrait => MipiOrientation::new()
                .rotate(Rotation::Deg180)
                .flip_horizontal(),
            // todo000 verify on device; provisional 180° rotation of Landscape.
            Orientation::LandscapeInverted => MipiOrientation::new()
                .rotate(Rotation::Deg90)
                .flip_horizontal(),
            // todo000 verify on device; provisional 180° rotation of Portrait.
            Orientation::PortraitInverted => MipiOrientation::new().flip_horizontal(),
        };

        let display = Builder::new(ILI9341Rgb565, interface)
            .reset_pin(rst)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .orientation(display_orientation)
            .init(&mut delay)
            .map_err(|_| CydDisplayEspInitError::InitDisplay)?;

        backlight.set_high();

        Ok(CydDisplayEsp {
            display,
            screen_size,
        })
    }

    pub(crate) fn flush_buffer(
        &mut self,
        buffer: &impl RectanglePixels,
        top_left: Point,
    ) -> Result<(), CydDisplayEspFlushError> {
        let rectangle = Rectangle::new(
            top_left,
            Size::new(buffer.width() as u32, buffer.height() as u32),
        );
        self.display
            .fill_contiguous(
                &rectangle,
                buffer
                    .raw_pixels()
                    .iter()
                    .copied()
                    .map(|pixel| Rgb565::from(RawU16::new(pixel))),
            )
            .map_err(|_| CydDisplayEspFlushError::FlushFrameBuffer)
    }

    pub(crate) fn make_frame_with_tile_top_left<'a>(
        &'a mut self,
        pixel_buffer: &'a mut dyn DynPixelBuffer,
        rectangle: Rectangle,
        tile_top_left: Point,
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
            tile_top_left,
            foreground565,
            font,
        }
    }

    pub(crate) fn fill(&mut self, color: Rgb565) -> Result<(), CydDisplayEspFlushError> {
        self.fill_rectangle(self.screen_rectangle(), color)
    }

    pub(crate) fn fill_rectangle(
        &mut self,
        rectangle: Rectangle,
        color: Rgb565,
    ) -> Result<(), CydDisplayEspFlushError> {
        self.display
            .fill_solid(&rectangle, color)
            .map_err(|_| CydDisplayEspFlushError::FlushFrameBuffer)
    }

    pub(crate) fn fill_contiguous<I>(
        &mut self,
        rectangle: Rectangle,
        pixels: I,
    ) -> Result<(), CydDisplayEspFlushError>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return Ok(());
        }
        self.display
            .fill_contiguous(&rectangle, pixels)
            .map_err(|_| CydDisplayEspFlushError::FlushFrameBuffer)
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
        rst_pin: impl OutputPin + 'static,
        backlight_pin: impl OutputPin + 'static,
        display_spi_hz: u32,
        orientation: Orientation,
    ) -> Result<CydDisplayEsp<CydDisplaySpiDevice>, CydDisplayEspInitError> {
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(display_spi_hz))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)
            .map_err(|_| CydDisplayEspInitError::ConfigureDisplaySpi)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        let cs = Output::new(cs_pin, Level::High, OutputConfig::default());

        let spi_device = ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs)
            .map_err(|_| CydDisplayEspInitError::CreateDisplaySpiDevice)?;

        Self::new_from_device(spi_device, dc_pin, rst_pin, backlight_pin, orientation)
    }
}
