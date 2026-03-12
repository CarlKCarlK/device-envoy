//! Embedded compile-only test target for one RFID reader on ESP.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    init_and_start::rmt_mode,
    rfid::{Rfid as _, RfidEsp, RfidStatic},
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

    let _ = rfid.wait_for_tap();

    core::future::pending().await
}
