//! A device abstraction for RFID readers using the MFRC522 chip.
//!
//! See [`RfidEsp`] for the primary example.
//!
//! You can create up to two concurrent `RfidEsp` instances per program; a third is expected to fail at runtime because the `rfid` task pool uses `pool_size = 2`.

pub use device_envoy_core::rfid::{Rfid, RfidEvent};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel as EmbassyChannel;
use embassy_time::{Instant, Timer};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::gpio::interconnect::{PeripheralInput, PeripheralOutput};
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};
use esp_hal::spi::master::{Config as SpiConfig, Instance, Spi};
use esp_hal_mfrc522::consts::UidSize;
use esp_hal_mfrc522::drivers::SpiDriver;
use esp_hal_mfrc522::MFRC522;

use crate::{Error, Result};

type Mfrc522Device =
    MFRC522<SpiDriver<ExclusiveDevice<Spi<'static, esp_hal::Async>, Output<'static>, NoDelay>>>;

/// Static resources for the [`RfidEsp`] device abstraction.
pub struct RfidStatic(EmbassyChannel<CriticalSectionRawMutex, RfidEvent, 4>);

impl RfidStatic {
    /// Create static resources for an RFID reader.
    #[must_use]
    pub const fn new() -> Self {
        Self(EmbassyChannel::new())
    }

    async fn send(&self, event: RfidEvent) {
        self.0.send(event).await;
    }

    async fn receive(&self) -> RfidEvent {
        self.0.receive().await
    }
}

/// A device abstraction for an RFID reader using the MFRC522 chip.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_esp::{
///     Result, init_and_start,
///     rfid::{Rfid, RfidEsp, RfidEvent, RfidStatic},
/// };
/// # use esp_backtrace as _;
/// # use log::info;
///
/// #[esp_rtos::main]
/// async fn main(spawner: embassy_executor::Spawner) -> ! {
///     let err = example(spawner).await.unwrap_err();
///     panic!("{err:?}");
/// }
///
/// async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
///     init_and_start!(p);
///     static RFID_STATIC: RfidStatic = RfidEsp::new_static();
///
///     let rfid = RfidEsp::new(
///         &RFID_STATIC,
///         p.SPI2,
///         p.GPIO6,
///         p.GPIO7,
///         p.GPIO2,
///         p.GPIO10,
///         p.GPIO5,
///         spawner,
///     )
///     .await?;
///
///     loop {
///         let RfidEvent::CardDetected { uid } = rfid.wait_for_tap().await;
///         info!("RFID uid: {:?}", uid);
///     }
/// }
/// ```
pub struct RfidEsp<'a> {
    rfid_static: &'a RfidStatic,
}

impl RfidEsp<'_> {
    /// Create static channel resources for an RFID reader.
    #[must_use]
    pub const fn new_static() -> RfidStatic {
        RfidStatic::new()
    }

    /// Create a new RFID reader device abstraction.
    ///
    /// See the [Rfid struct example](Self) for usage.
    pub async fn new(
        rfid_static: &'static RfidStatic,
        spi: impl Instance + 'static,
        sck: impl PeripheralOutput<'static>,
        mosi: impl PeripheralOutput<'static>,
        miso: impl PeripheralInput<'static>,
        cs: impl OutputPin + 'static,
        rst: impl OutputPin + 'static,
        spawner: Spawner,
    ) -> Result<Self> {
        let mfrc522 = init_mfrc522_hardware(spi, sck, mosi, miso, cs, rst).await?;
        let token = rfid_polling_task(mfrc522, rfid_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self { rfid_static })
    }
}

impl Rfid for RfidEsp<'_> {
    async fn wait_for_tap(&self) -> RfidEvent {
        self.rfid_static.receive().await
    }
}

fn uid_to_fixed_array(uid_bytes: &[u8]) -> [u8; 10] {
    let mut uid_key = [0u8; 10];
    for (uid_index, &uid_byte) in uid_bytes.iter().enumerate() {
        if uid_index < uid_key.len() {
            uid_key[uid_index] = uid_byte;
        }
    }
    uid_key
}

#[embassy_executor::task(pool_size = 2)]
async fn rfid_polling_task(mut mfrc522: Mfrc522Device, rfid_static: &'static RfidStatic) -> ! {
    loop {
        let Ok(()) = mfrc522.picc_is_new_card_present().await else {
            Timer::after_millis(500).await;
            continue;
        };

        let Ok(uid) = mfrc522.get_card(UidSize::Four).await else {
            Timer::after_millis(500).await;
            continue;
        };

        let uid_key = uid_to_fixed_array(&uid.uid_bytes);
        rfid_static
            .send(RfidEvent::CardDetected { uid: uid_key })
            .await;
        Timer::after_millis(50).await;
    }
}

async fn init_mfrc522_hardware(
    spi: impl Instance + 'static,
    sck: impl PeripheralOutput<'static>,
    mosi: impl PeripheralOutput<'static>,
    miso: impl PeripheralInput<'static>,
    cs: impl OutputPin + 'static,
    rst: impl OutputPin + 'static,
) -> Result<Mfrc522Device> {
    let spi_config = SpiConfig::default()
        .with_frequency(esp_hal::time::Rate::from_hz(1_000_000))
        .with_mode(esp_hal::spi::Mode::_0);
    let spi = Spi::new(spi, spi_config)
        .map_err(Error::SpiConfig)?
        .with_sck(sck)
        .with_mosi(mosi)
        .with_miso(miso)
        .into_async();

    let cs = Output::new(cs, Level::High, OutputConfig::default());

    let mut rst = Output::new(rst, Level::High, OutputConfig::default());
    rst.set_low();
    Timer::after_millis(10).await;
    rst.set_high();
    Timer::after_millis(50).await;

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).expect("CS pin is infallible");
    let spi_driver = SpiDriver::new(spi_device);
    let mut mfrc522 = MFRC522::new(spi_driver, || Instant::now().as_millis());

    mfrc522.pcd_init().await.map_err(Error::Mfrc522Init)?;
    let _version = mfrc522
        .pcd_get_version()
        .await
        .map_err(Error::Mfrc522Version)?;

    Ok(mfrc522)
}
