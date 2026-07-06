use device_envoy_core::cyd::RawTouchEvent;
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{
    Blocking, ClkPin, Config as SpiConfig, MisoPin, MosiPin, Phase, Polarity, Spi,
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

/// SPI clock frequency for the touch bus.
pub const TOUCH_SPI_HZ: u32 = 2_500_000;

type CydTouchSpiBus = Spi<'static, SPI1, Blocking>;
type CydTouchSpiDevice = ExclusiveDevice<CydTouchSpiBus, Output<'static>, NoDelay>;

/// Error initializing the touch controller over SPI.
#[derive(Clone, Copy, Debug)]
pub enum CydTouchRpInitError {
    /// Wrapping the touch SPI bus with its CS pin failed.
    CreateTouchSpiDevice,
}

pub(crate) struct CydTouchRp {
    touch_spi_device: CydTouchSpiDevice,
    touch_input: Xpt2046TouchInput<Input<'static>>,
}

impl CydTouchRp {
    pub(crate) fn new<Sck, Mosi, Miso, Cs, Irq>(
        spi: Peri<'static, SPI1>,
        sck_pin: Peri<'static, Sck>,
        mosi_pin: Peri<'static, Mosi>,
        miso_pin: Peri<'static, Miso>,
        cs_pin: Peri<'static, Cs>,
        irq_pin: Peri<'static, Irq>,
    ) -> Result<CydTouchRp, CydTouchRpInitError>
    where
        Sck: Pin + ClkPin<SPI1>,
        Mosi: Pin + MosiPin<SPI1>,
        Miso: Pin + MisoPin<SPI1>,
        Cs: Pin,
        Irq: Pin,
    {
        let spi_config = {
            let mut spi_config = SpiConfig::default();
            spi_config.frequency = TOUCH_SPI_HZ;
            spi_config.polarity = Polarity::IdleLow;
            spi_config.phase = Phase::CaptureOnFirstTransition;
            spi_config
        };
        let spi = Spi::new_blocking(spi, sck_pin, mosi_pin, miso_pin, spi_config);

        let cs = Output::new(cs_pin, Level::High);
        let irq = Input::new(irq_pin, Pull::Up);

        let touch_spi_device = ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs)
            .map_err(|_| CydTouchRpInitError::CreateTouchSpiDevice)?;
        let touch_input = Xpt2046TouchInput::new(irq);

        Ok(CydTouchRp {
            touch_spi_device,
            touch_input,
        })
    }

    pub(crate) fn read_raw_touch_event(&mut self) -> Option<RawTouchEvent> {
        self.touch_input
            .read_raw_touch_event(&mut self.touch_spi_device)
    }
}

/// Reads XPT2046 touch samples over an SPI device, using `TouchIrq` (the
/// controller's active-low interrupt pin) to detect press/release.
pub struct Xpt2046TouchInput<TouchIrq> {
    touch_irq: TouchIrq,
    was_pressed: bool,
}

impl<TouchIrq> Xpt2046TouchInput<TouchIrq>
where
    TouchIrq: embedded_hal::digital::InputPin,
{
    /// Create a new touch reader watching `touch_irq`.
    pub fn new(touch_irq: TouchIrq) -> Self {
        Self {
            touch_irq,
            was_pressed: false,
        }
    }

    fn is_pressed(&mut self) -> bool {
        self.touch_irq.is_low().unwrap_or(false)
    }

    fn read_single_axis(
        touch_spi_device: &mut impl embedded_hal::spi::SpiDevice<u8>,
        command: u8,
    ) -> u16 {
        let tx = [command, 0x00, 0x00];
        let mut rx = [0u8; 3];
        touch_spi_device
            .transfer(&mut rx, &tx)
            .expect("touch axis SPI failed");
        (((rx[1] as u16) << 8) | (rx[2] as u16)) >> 3
    }

    fn read_single_xy(
        touch_spi_device: &mut impl embedded_hal::spi::SpiDevice<u8>,
    ) -> Option<(u16, u16)> {
        let raw_x = Self::read_single_axis(touch_spi_device, 0xD0);
        let raw_y = Self::read_single_axis(touch_spi_device, 0x90);

        if raw_x > 0 && raw_y > 0 {
            Some((raw_x, raw_y))
        } else {
            None
        }
    }

    fn read_raw_xy(
        &mut self,
        touch_spi_device: &mut impl embedded_hal::spi::SpiDevice<u8>,
    ) -> Option<(u16, u16)> {
        const SAMPLES: u32 = 3;
        // TODO probe-level median-of-N may reject raw ADC outliers better than
        // this plain average, but calibration-flow release averaging is the
        // current fix of record (may no longer apply; mirrors the same note
        // on the esp32 CydEsp equivalent).
        let mut sum_x: u32 = 0;
        let mut sum_y: u32 = 0;
        let mut count: u32 = 0;
        for _ in 0..SAMPLES {
            if let Some((x, y)) = Self::read_single_xy(touch_spi_device) {
                sum_x += x as u32;
                sum_y += y as u32;
                count += 1;
            }
        }
        if count > 0 {
            let avg_x = (sum_x / count) as u16;
            let avg_y = (sum_y / count) as u16;
            Some((avg_x, avg_y))
        } else {
            None
        }
    }

    /// Poll for the next raw touch event, if any, over `touch_spi_device`.
    pub fn read_raw_touch_event(
        &mut self,
        touch_spi_device: &mut impl embedded_hal::spi::SpiDevice<u8>,
    ) -> Option<RawTouchEvent> {
        let touch_is_pressed = self.is_pressed();

        if touch_is_pressed {
            if let Some((raw_x, raw_y)) = self.read_raw_xy(touch_spi_device) {
                let event = if self.was_pressed {
                    RawTouchEvent::Move { raw_x, raw_y }
                } else {
                    RawTouchEvent::Down { raw_x, raw_y }
                };

                self.was_pressed = true;
                return Some(event);
            }
        } else if self.was_pressed {
            self.was_pressed = false;
            return Some(RawTouchEvent::Up);
        }

        None
    }
}
