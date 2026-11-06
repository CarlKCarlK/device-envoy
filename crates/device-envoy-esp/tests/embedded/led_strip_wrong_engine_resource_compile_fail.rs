//! Embedded compile-fail test target for a strip given the wrong engine's peripheral.
//!
//! An `Rmt` strip is passed an SPI peripheral, so `SPI2` does not implement
//! `EngineResource<Rmt>` and the build must fail with that trait's message.
//! `led_strip_engine_trait_compile` is the passing counterpart.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, led_strip, led_strip::Rmt};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    RmtStrip { pin: GPIO1, len: 4, engine: Rmt }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);
    let _rmt_strip = RmtStrip::new(p.GPIO1, p.SPI2, spawner)?;
    core::future::pending().await
}
