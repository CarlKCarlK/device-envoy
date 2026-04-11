//! Embedded compile-only test target for one RFID reader on ESP.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    rfid::{Rfid as _, RfidEsp, RfidStatic},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    static RFID_STATIC: RfidStatic = RfidEsp::new_static();

    init_and_start!(p);

    #[cfg(any(feature = "esp32", feature = "esp32h2"))]
    let rfid = RfidEsp::new(
        &RFID_STATIC,
        p.SPI2,
        p.GPIO0,
        p.GPIO1,
        p.GPIO2,
        p.GPIO3,
        p.GPIO4,
        spawner,
    )
    .await?;

    #[cfg(not(any(feature = "esp32", feature = "esp32h2")))]
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
