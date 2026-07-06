use device_envoy_core::cyd::RegionPixels;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::MonoFont,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{Point, Size},
    primitives::Rectangle,
};
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

// 80 MHz measured 10.9 draw+flush fps but produced visible display corruption.
/// SPI clock frequency for the display bus.
pub const DISPLAY_SPI_HZ: u32 = 60_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

type CydDisplaySpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
type CydDisplaySpiDevice = ExclusiveDevice<CydDisplaySpiBus, Output<'static>, NoDelay>;
type CydDisplayInterface = SpiInterface<'static, CydDisplaySpiDevice, Output<'static>>;
type CydDisplayDevice = mipidsi::Display<CydDisplayInterface, ILI9341Rgb565, Output<'static>>;

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

pub(crate) struct CydDisplayEsp {
    display: CydDisplayDevice,
    screen_size: Size,
}

impl CydDisplayEsp {
    /// Oriented screen size stored at init time.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.screen_size
    }

    #[must_use]
    fn screen_rectangle(&self) -> Rectangle {
        Rectangle::new(Point::new(0, 0), self.screen_size)
    }

    // TODO0000 Revisit whether this software clipping should stay here or be
    // delegated to panel hardware/windowing behavior after measuring the real
    // controller semantics and cost.
    #[must_use]
    fn clip_to_screen(&self, rectangle: Rectangle) -> Option<Rectangle> {
        let rectangle = rectangle.intersection(&self.screen_rectangle());
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return None;
        }
        Some(rectangle)
    }

    pub(crate) fn new(
        spi: impl spi::master::Instance + 'static,
        sck_pin: impl PeripheralOutput<'static>,
        mosi_pin: impl PeripheralOutput<'static>,
        miso_pin: impl PeripheralInput<'static>,
        cs_pin: impl OutputPin + 'static,
        dc_pin: impl OutputPin + 'static,
        rst_pin: impl OutputPin + 'static,
        backlight_pin: impl OutputPin + 'static,
        orientation: Orientation,
    ) -> Result<CydDisplayEsp, CydDisplayEspInitError> {
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(DISPLAY_SPI_HZ))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)
            .map_err(|_| CydDisplayEspInitError::ConfigureDisplaySpi)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        let cs = Output::new(cs_pin, Level::High, OutputConfig::default());
        let dc = Output::new(dc_pin, Level::Low, OutputConfig::default());
        let rst = Output::new(rst_pin, Level::High, OutputConfig::default());
        let mut backlight = Output::new(backlight_pin, Level::High, OutputConfig::default());

        let spi_device = ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs)
            .map_err(|_| CydDisplayEspInitError::CreateDisplaySpiDevice)?;

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
        buffer: &impl RegionPixels,
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
    ) -> CydFrameEsp<'a> {
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
        let Some(rectangle) = self.clip_to_screen(rectangle) else {
            return Ok(());
        };
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
