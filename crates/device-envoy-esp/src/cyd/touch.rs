use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    gpio::{
        Input, InputConfig, InputPin as EspInputPin, Output, OutputConfig, OutputPin, Pull,
        interconnect::{PeripheralInput, PeripheralOutput},
    },
    spi,
};
use device_envoy_core::cyd::RawTouchEvent;

pub const TOUCH_SPI_HZ: u32 = 2_500_000;

type CydTouchSpiBus = spi::master::Spi<'static, esp_hal::Blocking>;
type CydTouchSpiDevice = ExclusiveDevice<CydTouchSpiBus, Output<'static>, NoDelay>;

#[derive(Clone, Copy, Debug)]
pub enum CydTouchEspInitError {
    ConfigureTouchSpi,
    CreateTouchSpiDevice,
}

pub(crate) struct CydTouchEsp {
    touch_spi_device: CydTouchSpiDevice,
    touch_input: Xpt2046TouchInput<Input<'static>>,
}

impl CydTouchEsp {
    pub(crate) fn new(
        spi: impl spi::master::Instance + 'static,
        sck_pin: impl PeripheralOutput<'static>,
        mosi_pin: impl PeripheralOutput<'static>,
        miso_pin: impl PeripheralInput<'static>,
        cs_pin: impl OutputPin + 'static,
        irq_pin: impl EspInputPin + 'static,
    ) -> Result<CydTouchEsp, CydTouchEspInitError> {
        let spi_config = spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(TOUCH_SPI_HZ))
            .with_mode(spi::Mode::_0);
        let spi = spi::master::Spi::new(spi, spi_config)
            .map_err(|_| CydTouchEspInitError::ConfigureTouchSpi)?
            .with_sck(sck_pin)
            .with_mosi(mosi_pin)
            .with_miso(miso_pin);

        let cs = Output::new(cs_pin, esp_hal::gpio::Level::High, OutputConfig::default());
        let irq = Input::new(irq_pin, InputConfig::default().with_pull(Pull::Up));

        let touch_spi_device = ExclusiveDevice::<_, _, NoDelay>::new_no_delay(spi, cs)
            .map_err(|_| CydTouchEspInitError::CreateTouchSpiDevice)?;
        let touch_input = Xpt2046TouchInput::new(irq);

        Ok(CydTouchEsp {
            touch_spi_device,
            touch_input,
        })
    }

    pub(crate) fn read_raw_touch_event(&mut self) -> Option<RawTouchEvent> {
        self.touch_input
            .read_raw_touch_event(&mut self.touch_spi_device)
    }
}

pub struct Xpt2046TouchInput<TouchIrq> {
    touch_irq: TouchIrq,
    was_pressed: bool,
}

impl<TouchIrq> Xpt2046TouchInput<TouchIrq>
where
    TouchIrq: embedded_hal::digital::InputPin,
{
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
        // current fix of record (may no longer apply).
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
