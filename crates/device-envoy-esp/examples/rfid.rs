//! RFID reader example using an MFRC522 module.
//!
//! Wiring (ESP32-C6 defaults shown):
//! - SPI2 SCK  -> GPIO6
//! - SPI2 MOSI -> GPIO7
//! - SPI2 MISO -> GPIO2
//! - MFRC522 CS (SDA/SS) -> GPIO10
//! - MFRC522 RST          -> GPIO5
//! - Plus 3.3V and GND

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start,
    rfid::{Rfid as _, RfidEsp, RfidEvent, RfidStatic},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    static RFID_STATIC: RfidStatic = RfidEsp::new_static();

    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("rfid example started");

    let rfid = RfidEsp::new(
        &RFID_STATIC,
        p.SPI2,
        p.GPIO6,
        p.GPIO7,
        p.GPIO2,
        p.GPIO10,
        p.GPIO5,
        spawner,
    )
    .await?;

    loop {
        let RfidEvent::CardDetected { uid } = rfid.wait_for_tap().await;
        info!("RFID uid: {:?}", uid);
    }
}
