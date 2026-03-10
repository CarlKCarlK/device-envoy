//! A device abstraction for RFID readers using the MFRC522 chip.
//!
//! See [`Rfid`] for the primary example; helper methods link back there.

use defmt::info;
pub use device_envoy_core::rfid::{Rfid as RfidTrait, RfidEvent, RfidStatic};
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::{SPI0, SPI1};
use embassy_rp::spi::{
    Async, ClkPin, Config as SpiConfig, Instance, MisoPin, MosiPin, Phase, Polarity, Spi,
};
use embassy_time::{Instant, Timer};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal_mfrc522::MFRC522;
use esp_hal_mfrc522::consts::{PCDErrorCode, UidSize};
use esp_hal_mfrc522::drivers::SpiDriver;

use crate::{Error, Result};

type Mfrc522OnSpi<SPI> =
    MFRC522<SpiDriver<ExclusiveDevice<Spi<'static, SPI, Async>, Output<'static>, NoDelay>>>;
type Mfrc522Spi0Device = Mfrc522OnSpi<SPI0>;
type Mfrc522Spi1Device = Mfrc522OnSpi<SPI1>;

enum Mfrc522Device {
    Spi0(Mfrc522Spi0Device),
    Spi1(Mfrc522Spi1Device),
}

impl Mfrc522Device {
    async fn picc_is_new_card_present(&mut self) -> core::result::Result<(), PCDErrorCode> {
        match self {
            Self::Spi0(device) => device.picc_is_new_card_present().await,
            Self::Spi1(device) => device.picc_is_new_card_present().await,
        }
    }

    async fn get_card(
        &mut self,
        uid_size: UidSize,
    ) -> core::result::Result<esp_hal_mfrc522::consts::Uid, PCDErrorCode> {
        match self {
            Self::Spi0(device) => device.get_card(uid_size).await,
            Self::Spi1(device) => device.get_card(uid_size).await,
        }
    }
}

/// A device abstraction for an RFID reader using the MFRC522 chip.
///
/// ```rust,no_run
/// # #![no_std]
/// # use panic_probe as _;
/// # use defmt::info;
/// # fn main() {}
/// use device_envoy_rp::rfid::{Rfid, RfidEvent, RfidStatic, RfidTrait as _};
///
/// async fn example(
///     p: embassy_rp::Peripherals,
///     spawner: embassy_executor::Spawner,
/// ) -> device_envoy_rp::Result<()> {
///     static RFID_STATIC: RfidStatic = Rfid::new_static();
///     let rfid = Rfid::new_spi0(
///         &RFID_STATIC,
///         p.SPI0,
///         p.PIN_2,
///         p.PIN_3,
///         p.PIN_4,
///         p.DMA_CH0,
///         p.DMA_CH1,
///         p.PIN_1,
///         p.PIN_5,
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
pub struct Rfid<'a> {
    rfid_static: &'a RfidStatic,
}

impl Rfid<'_> {
    /// Create static channel resources for an RFID reader.
    #[must_use]
    pub const fn new_static() -> RfidStatic {
        RfidStatic::new()
    }

    /// Create a new RFID reader instance using SPI0.
    ///
    /// See the [Rfid struct example](Self) for usage.
    pub async fn new_spi0<Sck, Mosi, Miso, DmaTx, DmaRx, Cs, Rst>(
        rfid_static: &'static RfidStatic,
        spi: Peri<'static, SPI0>,
        sck: Peri<'static, Sck>,
        mosi: Peri<'static, Mosi>,
        miso: Peri<'static, Miso>,
        dma_tx: Peri<'static, DmaTx>,
        dma_rx: Peri<'static, DmaRx>,
        cs: Peri<'static, Cs>,
        rst: Peri<'static, Rst>,
        spawner: Spawner,
    ) -> Result<Self>
    where
        Sck: Pin + ClkPin<SPI0>,
        Mosi: Pin + MosiPin<SPI0>,
        Miso: Pin + MisoPin<SPI0>,
        DmaTx: Channel,
        DmaRx: Channel,
        Cs: Pin,
        Rst: Pin,
    {
        let mfrc522 = init_mfrc522_hardware(spi, sck, mosi, miso, dma_tx, dma_rx, cs, rst).await?;
        Self::new_with_device(rfid_static, Mfrc522Device::Spi0(mfrc522), spawner)
    }

    /// Create a new RFID reader instance using SPI1.
    ///
    /// See the [Rfid struct example](Self) for usage.
    pub async fn new_spi1<Sck, Mosi, Miso, DmaTx, DmaRx, Cs, Rst>(
        rfid_static: &'static RfidStatic,
        spi: Peri<'static, SPI1>,
        sck: Peri<'static, Sck>,
        mosi: Peri<'static, Mosi>,
        miso: Peri<'static, Miso>,
        dma_tx: Peri<'static, DmaTx>,
        dma_rx: Peri<'static, DmaRx>,
        cs: Peri<'static, Cs>,
        rst: Peri<'static, Rst>,
        spawner: Spawner,
    ) -> Result<Self>
    where
        Sck: Pin + ClkPin<SPI1>,
        Mosi: Pin + MosiPin<SPI1>,
        Miso: Pin + MisoPin<SPI1>,
        DmaTx: Channel,
        DmaRx: Channel,
        Cs: Pin,
        Rst: Pin,
    {
        let mfrc522 = init_mfrc522_hardware(spi, sck, mosi, miso, dma_tx, dma_rx, cs, rst).await?;
        Self::new_with_device(rfid_static, Mfrc522Device::Spi1(mfrc522), spawner)
    }

    fn new_with_device(
        rfid_static: &'static RfidStatic,
        mfrc522: Mfrc522Device,
        spawner: Spawner,
    ) -> Result<Self> {
        let token = rfid_polling_task(mfrc522, rfid_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self { rfid_static })
    }
}

impl RfidTrait for Rfid<'_> {
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
    info!("RFID polling task started");

    loop {
        let Ok(()) = mfrc522.picc_is_new_card_present().await else {
            Timer::after_millis(500).await;
            continue;
        };

        info!("Card detected!");

        let Ok(uid) = mfrc522.get_card(UidSize::Four).await else {
            info!("UID read error");
            Timer::after_millis(500).await;
            continue;
        };

        let uid_key = uid_to_fixed_array(&uid.uid_bytes);
        rfid_static.send(RfidEvent::CardDetected { uid: uid_key }).await;

        Timer::after_millis(50).await;
    }
}

async fn init_mfrc522_hardware<SPI, Sck, Mosi, Miso, DmaTx, DmaRx, Cs, Rst>(
    spi: Peri<'static, SPI>,
    sck: Peri<'static, Sck>,
    mosi: Peri<'static, Mosi>,
    miso: Peri<'static, Miso>,
    dma_tx: Peri<'static, DmaTx>,
    dma_rx: Peri<'static, DmaRx>,
    cs: Peri<'static, Cs>,
    rst: Peri<'static, Rst>,
) -> Result<Mfrc522OnSpi<SPI>>
where
    SPI: Instance,
    Sck: Pin + ClkPin<SPI>,
    Mosi: Pin + MosiPin<SPI>,
    Miso: Pin + MisoPin<SPI>,
    DmaTx: Channel,
    DmaRx: Channel,
    Cs: Pin,
    Rst: Pin,
{
    let spi = Spi::new(spi, sck, mosi, miso, dma_tx, dma_rx, {
        let mut spi_config = SpiConfig::default();
        spi_config.frequency = 1_000_000;
        spi_config.polarity = Polarity::IdleLow;
        spi_config.phase = Phase::CaptureOnFirstTransition;
        spi_config
    });

    let cs = Output::new(cs, Level::High);

    let mut rst = Output::new(rst, Level::High);
    rst.set_low();
    Timer::after_millis(10).await;
    rst.set_high();
    Timer::after_millis(50).await;

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).expect("CS pin is infallible");
    let spi_driver = SpiDriver::new(spi_device);
    let mut mfrc522 = MFRC522::new(spi_driver, || Instant::now().as_millis());

    mfrc522.pcd_init().await.map_err(Error::Mfrc522Init)?;
    info!("MFRC522 initialized");

    let _version = mfrc522
        .pcd_get_version()
        .await
        .map_err(Error::Mfrc522Version)?;
    info!("MFRC522 version read successfully");

    Ok(mfrc522)
}
