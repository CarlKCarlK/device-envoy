//! Embedded compile-only test target for two ClockSyncEsp instances.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::Stack;
use esp_backtrace as _;

use device_envoy_esp::clock_sync::{ClockSyncEsp, ClockSyncStaticEsp, ONE_SECOND};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    let _ = spawner;
    core::future::pending().await
}

#[allow(dead_code, reason = "Compile-time verification only")]
fn test_two_clock_syncs(stack: &'static Stack<'static>, spawner: Spawner) {
    static CLOCK_SYNC0_STATIC: ClockSyncStaticEsp = ClockSyncEsp::new_static();
    static CLOCK_SYNC1_STATIC: ClockSyncStaticEsp = ClockSyncEsp::new_static();

    let _clock_sync0 = ClockSyncEsp::new(&CLOCK_SYNC0_STATIC, stack, 0, Some(ONE_SECOND), spawner);
    let _clock_sync1 = ClockSyncEsp::new(&CLOCK_SYNC1_STATIC, stack, 0, Some(ONE_SECOND), spawner);
}
