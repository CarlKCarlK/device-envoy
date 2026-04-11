//! Embedded compile-fail test target for deprecated IR colon syntax.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::ir;

esp_bootloader_esp_idf::esp_app_desc!();

ir! {
    IrOldStyle: { pin: GPIO0 }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    core::future::pending().await
}
