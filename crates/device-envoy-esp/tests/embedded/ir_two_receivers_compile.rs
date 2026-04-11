//! Embedded compile-only test target for two IR receivers.
//!
//! This test target should build for all RMT-capable chips. On hardware, construction
//! of both receivers must succeed with distinct channels.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplersCompileTest {
        IrKepler0: { pin: GPIO0 },
        IrKepler1: { pin: GPIO1 }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    #[cfg(feature = "esp32s2")]
    let channel_creator0 = rmt80.channel0;
    #[cfg(all(target_arch = "xtensa", not(feature = "esp32s2")))]
    let channel_creator0 = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator0 = rmt80.channel2;

    #[cfg(feature = "esp32s2")]
    let channel_creator1 = rmt80.channel1;
    #[cfg(all(target_arch = "xtensa", not(feature = "esp32s2")))]
    let channel_creator1 = rmt80.channel5;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator1 = rmt80.channel3;

    let (_ir_kepler0, _ir_kepler1) = IrKeplersCompileTest::new(
        p.GPIO0,
        channel_creator0,
        p.GPIO1,
        channel_creator1,
        spawner,
    )?;

    core::future::pending().await
}
