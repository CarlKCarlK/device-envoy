#![allow(missing_docs)]
//! RFID reader example using an MFRC522 module.
//!
//! Wiring:
//!
//! - SPI0 SCK       -> PIN_2
//! - SPI0 MOSI      -> PIN_3
//! - SPI0 MISO      -> PIN_4
//! - MFRC522 CS (SDA/SS) -> PIN_1
//! - MFRC522 RST    -> PIN_5
//! - Plus 3.3V and GND

#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{
    Result,
    rfid::{Rfid, RfidEvent, RfidRp, RfidStatic},
};
use embassy_executor::Spawner;
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("rfid example started");

    static RFID_STATIC: RfidStatic = RfidRp::new_static();
    let rfid = RfidRp::new_spi0(
        &RFID_STATIC,
        p.SPI0,
        p.PIN_2,
        p.PIN_3,
        p.PIN_4,
        p.DMA_CH0,
        p.DMA_CH1,
        p.PIN_1,
        p.PIN_5,
        spawner,
    )
    .await?;

    loop {
        let RfidEvent::CardDetected { uid } = rfid.wait_for_tap().await;
        info!("RFID uid: {:?}", uid);
    }
}
