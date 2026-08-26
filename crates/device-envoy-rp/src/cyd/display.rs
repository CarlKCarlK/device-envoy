use device_envoy_core::UnwrapInfallible;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::SPI0;
use embassy_rp::spi::{
    Blocking, ClkPin, Config as SpiConfig, MisoPin, MosiPin, Phase, Polarity, Spi,
};
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::MonoFont,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{Point, Size},
    primitives::Rectangle,
};
use embedded_hal::spi::SpiDevice;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ST7789,
    options::{ColorOrder, Orientation as MipiOrientation, Rotation},
};
use static_cell::StaticCell;

use super::{CydFrameRp, Orientation, buffer::DynPixelBuffer};

// 80 MHz is the current tested default for CYD display traffic on RP: it is
// materially faster than bring-up speeds while still stable on the hardware
// used so far.
/// Default SPI clock for CYD display traffic on RP boards.
///
/// See the [`CydDisplayRp::new`](super::CydDisplayRp::new) constructor example.
pub const DEFAULT_DISPLAY_SPI_HZ: u32 = 40_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

type CydDisplaySpiBus = Spi<'static, SPI0, Blocking>;
/// The SPI device type used when the display owns an exclusive SPI peripheral.
pub(crate) type CydDisplaySpiDevice = ExclusiveDevice<CydDisplaySpiBus, Output<'static>, NoDelay>;
type CydDisplayInterface<D> = SpiInterface<'static, D, Output<'static>>;
type CydDisplayDevice<D> = mipidsi::Display<CydDisplayInterface<D>, ST7789, Output<'static>>;

/// An ST7789 display driven over `D`, an `embedded-hal` SPI device.
///
/// `D` defaults to [`CydDisplaySpiDevice`], an exclusively-owned SPI
/// peripheral. Shared-bus backends (see `one_spi`) instead construct this
/// with an `embassy_embedded_hal::shared_bus` device via
/// [`CydDisplayRp::new_from_device`].
pub(crate) struct CydDisplayRp<D: SpiDevice<u8> = CydDisplaySpiDevice> {
    display: CydDisplayDevice<D>,
    screen_size: Size,
    _backlight: Output<'static>,
}

impl<D: SpiDevice<u8>> CydDisplayRp<D> {
    pub(crate) fn set_orientation(&mut self, orientation: Orientation) -> Result<(), super::Error> {
        let display_orientation = match orientation {
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
        };
        self.display
            .set_orientation(display_orientation)
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
    /// example an `embassy_embedded_hal::shared_bus` device wrapping a bus
    /// shared with touch) instead of owning an exclusive SPI peripheral.
    pub(crate) fn new_from_device<Dc, Rst, Backlight>(
        spi_device: D,
        dc_pin: Peri<'static, Dc>,
        rst_pin: Peri<'static, Rst>,
        backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
    ) -> Result<CydDisplayRp<D>, super::Error>
    where
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let dc = Output::new(dc_pin, Level::Low);
        let rst = Output::new(rst_pin, Level::High);
        let mut backlight = Output::new(backlight_pin, Level::High);

        static SPI_BUFFER: StaticCell<[u8; DISPLAY_SPI_BUFFER_LEN]> = StaticCell::new();
        let spi_buffer = SPI_BUFFER.init([0u8; DISPLAY_SPI_BUFFER_LEN]);
        let interface = SpiInterface::new(spi_device, dc, spi_buffer);
        let mut delay = embassy_time::Delay;

        let screen_size = orientation.size();
        let display_orientation = match orientation {
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
        };

        let display = Builder::new(ST7789, interface)
            .reset_pin(rst)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .orientation(display_orientation)
            .init(&mut delay)
            // `InitError` is parameterized by the private SPI-device type. The
            // public CYD error keeps the operation context at this boundary.
            .map_err(|_| super::Error::InitDisplay)?;

        backlight.set_high();

        Ok(CydDisplayRp {
            display,
            screen_size,
            _backlight: backlight,
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

    pub(crate) fn make_frame_with_tile_top_left<'a>(
        &'a mut self,
        pixel_buffer: &'a mut dyn DynPixelBuffer,
        rectangle: Rectangle,
        tile_top_left: Point,
        background565: Rgb565,
        foreground565: Rgb565,
        font: &'static MonoFont<'static>,
    ) -> CydFrameRp<'a, D> {
        let size = rectangle.size;
        let mut view = pixel_buffer.view_mut(size.width as usize, size.height as usize);
        // Every new frame starts cleared to the device background so callers
        // never have to clear it themselves.
        view.fill(background565);
        CydFrameRp {
            display: self,
            view,
            rectangle,
            tile_top_left,
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

impl CydDisplayRp<CydDisplaySpiDevice> {
    #[expect(clippy::too_many_arguments, reason = "mirrors CydDisplayRp::new")]
    pub(crate) fn new<Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        spi: Peri<'static, SPI0>,
        sck_pin: Peri<'static, Sck>,
        mosi_pin: Peri<'static, Mosi>,
        _miso_pin: Peri<'static, Miso>,
        cs_pin: Peri<'static, Cs>,
        dc_pin: Peri<'static, Dc>,
        rst_pin: Peri<'static, Rst>,
        backlight_pin: Peri<'static, Backlight>,
        display_spi_hz: u32,
        orientation: Orientation,
    ) -> Result<CydDisplayRp<CydDisplaySpiDevice>, super::Error>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        Cs: Pin,
        Dc: Pin,
        Rst: Pin,
        Backlight: Pin,
    {
        let spi_config = {
            let mut spi_config = SpiConfig::default();
            spi_config.frequency = display_spi_hz;
            spi_config.polarity = Polarity::IdleLow;
            spi_config.phase = Phase::CaptureOnFirstTransition;
            spi_config
        };
        // The display path here is write-only. Driving SPI0 in TX-only mode
        // avoids relying on a display MISO line that may be absent, floating,
        // or incompatible on loose jumper-wire bring-up setups.
        let spi = Spi::new_blocking_txonly(spi, sck_pin, mosi_pin, spi_config);

        let cs = Output::new(cs_pin, Level::High);

        let spi_device =
            ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs).unwrap_infallible();

        Self::new_from_device(spi_device, dc_pin, rst_pin, backlight_pin, orientation)
    }
}
