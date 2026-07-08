use device_envoy_core::cyd::display::RectanglePixels;
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
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ST7789,
    options::{ColorOrder, Orientation as MipiOrientation, Rotation},
};
use static_cell::StaticCell;

use super::{CydFrameRp, Orientation, buffer::DynPixelBuffer};

// The CYD panel tolerates high SPI clocks on short PCB traces, but RP bring-up
// here often uses jumper wires between boards. Use a conservative rate first so
// display validation is not dominated by signal-integrity failures.
/// SPI clock frequency for the display bus.
pub const DISPLAY_SPI_HZ: u32 = 2_000_000;
const DISPLAY_SPI_BUFFER_LEN: usize = 64;

type CydDisplaySpiBus = Spi<'static, SPI0, Blocking>;
type CydDisplaySpiDevice = ExclusiveDevice<CydDisplaySpiBus, Output<'static>, NoDelay>;
type CydDisplayInterface = SpiInterface<'static, CydDisplaySpiDevice, Output<'static>>;
type CydDisplayDevice = mipidsi::Display<CydDisplayInterface, ST7789, Output<'static>>;

/// Error initializing the display over SPI.
#[derive(Clone, Copy, Debug)]
pub enum CydDisplayRpInitError {
    /// The mipidsi display driver failed to initialize the panel.
    InitDisplay,
}

/// Error flushing pixels to the display.
#[derive(Clone, Copy, Debug)]
pub enum CydDisplayRpFlushError {
    /// Writing the frame buffer to the panel over SPI failed.
    FlushFrameBuffer,
}

pub(crate) struct CydDisplayRp {
    display: CydDisplayDevice,
    screen_size: Size,
    _backlight: Output<'static>,
}

impl CydDisplayRp {
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
    // controller semantics and cost (mirrors the same open question on the
    // esp32 CydEsp equivalent).
    #[must_use]
    fn clip_to_screen(&self, rectangle: Rectangle) -> Option<Rectangle> {
        let rectangle = rectangle.intersection(&self.screen_rectangle());
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return None;
        }
        Some(rectangle)
    }

    pub(crate) fn new<Sck, Mosi, Miso, Cs, Dc, Rst, Backlight>(
        spi: Peri<'static, SPI0>,
        sck_pin: Peri<'static, Sck>,
        mosi_pin: Peri<'static, Mosi>,
        _miso_pin: Peri<'static, Miso>,
        cs_pin: Peri<'static, Cs>,
        dc_pin: Peri<'static, Dc>,
        rst_pin: Peri<'static, Rst>,
        backlight_pin: Peri<'static, Backlight>,
        orientation: Orientation,
    ) -> Result<CydDisplayRp, CydDisplayRpInitError>
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
            spi_config.frequency = DISPLAY_SPI_HZ;
            spi_config.polarity = Polarity::IdleLow;
            spi_config.phase = Phase::CaptureOnFirstTransition;
            spi_config
        };
        // The display path here is write-only. Driving SPI0 in TX-only mode
        // avoids relying on a display MISO line that may be absent, floating,
        // or incompatible on loose jumper-wire bring-up setups.
        let spi = Spi::new_blocking_txonly(spi, sck_pin, mosi_pin, spi_config);

        let cs = Output::new(cs_pin, Level::High);
        let dc = Output::new(dc_pin, Level::Low);
        let rst = Output::new(rst_pin, Level::High);
        let mut backlight = Output::new(backlight_pin, Level::High);

        let spi_device =
            ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs).expect("CS pin is infallible");

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
            // todo000 verify on device; provisional 180° rotation of Landscape.
            Orientation::LandscapeInverted => MipiOrientation::new()
                .rotate(Rotation::Deg90)
                .flip_horizontal(),
            // todo000 verify on device; provisional 180° rotation of Portrait.
            Orientation::PortraitInverted => MipiOrientation::new().flip_horizontal(),
        };

        let display = Builder::new(ST7789, interface)
            .reset_pin(rst)
            .display_size(240, 320)
            .color_order(ColorOrder::Bgr)
            .orientation(display_orientation)
            .init(&mut delay)
            .map_err(|_| CydDisplayRpInitError::InitDisplay)?;

        backlight.set_high();

        Ok(CydDisplayRp {
            display,
            screen_size,
            _backlight: backlight,
        })
    }

    pub(crate) fn flush_buffer(
        &mut self,
        buffer: &impl RectanglePixels,
        top_left: Point,
    ) -> Result<(), CydDisplayRpFlushError> {
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
            .map_err(|_| CydDisplayRpFlushError::FlushFrameBuffer)
    }

    pub(crate) fn make_frame_with_tile_top_left<'a>(
        &'a mut self,
        pixel_buffer: &'a mut dyn DynPixelBuffer,
        rectangle: Rectangle,
        tile_top_left: Point,
        background565: Rgb565,
        foreground565: Rgb565,
        font: &'static MonoFont<'static>,
    ) -> CydFrameRp<'a> {
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
            foreground565,
            font,
        }
    }

    pub(crate) fn fill(&mut self, color: Rgb565) -> Result<(), CydDisplayRpFlushError> {
        self.fill_rectangle(self.screen_rectangle(), color)
    }

    pub(crate) fn fill_rectangle(
        &mut self,
        rectangle: Rectangle,
        color: Rgb565,
    ) -> Result<(), CydDisplayRpFlushError> {
        let Some(rectangle) = self.clip_to_screen(rectangle) else {
            return Ok(());
        };
        self.display
            .fill_solid(&rectangle, color)
            .map_err(|_| CydDisplayRpFlushError::FlushFrameBuffer)
    }

    pub(crate) fn fill_contiguous<I>(
        &mut self,
        rectangle: Rectangle,
        pixels: I,
    ) -> Result<(), CydDisplayRpFlushError>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        if rectangle.size.width == 0 || rectangle.size.height == 0 {
            return Ok(());
        }
        self.display
            .fill_contiguous(&rectangle, pixels)
            .map_err(|_| CydDisplayRpFlushError::FlushFrameBuffer)
    }
}
