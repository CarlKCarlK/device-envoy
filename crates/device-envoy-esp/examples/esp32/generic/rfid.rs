//! Wiring:
//!
//! RFID reader example using an MFRC522 module.
//!
//! - SPI2 SCK  -> GPIO18
//! - SPI2 MOSI -> GPIO21
//! - SPI2 MISO -> GPIO19
//! - MFRC522 CS (SDA/SS) -> GPIO5
//! - MFRC522 RST -> GPIO4
//! - Plus 3.3V and GND

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start,
    rfid::{Rfid, RfidEsp, RfidEvent, RfidStatic},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    static RFID_STATIC: RfidStatic = RfidEsp::new_static();

    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("rfid example started");

    let rfid = RfidEsp::new(
        &RFID_STATIC,
        p.SPI2,
        p.GPIO18,
        p.GPIO21,
        p.GPIO19,
        p.GPIO5,
        p.GPIO4,
        spawner,
    )
    .await?;

    loop {
        let RfidEvent::CardDetected { uid } = rfid.wait_for_tap().await;
        info!("RFID uid: {:?}", uid);
    }
}
