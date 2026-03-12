//! Embedded compile-only test target for two IR receivers.
//!
//! This test target should build for both C6 and S3. On hardware, construction of
//! both receivers must succeed with distinct channels.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, init_and_start::rmt_mode, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplersCompileTest {
        IrKepler7: { pin: GPIO7 },
        IrKepler6: { pin: GPIO6 }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    #[cfg(target_arch = "xtensa")]
    let channel_creator0 = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator0 = rmt80.channel2;

    #[cfg(target_arch = "xtensa")]
    let channel_creator1 = rmt80.channel5;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator1 = rmt80.channel3;

    let (_ir_kepler7, _ir_kepler6) = IrKeplersCompileTest::new(
        p.GPIO7,
        channel_creator0,
        p.GPIO6,
        channel_creator1,
        spawner,
    )?;

    core::future::pending().await
}
