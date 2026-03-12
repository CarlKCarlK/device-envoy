#![allow(missing_docs)]
//! Compile-only verification that two ClockSyncRp instances can be created.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::clock_sync::{ClockSyncRp, ClockSyncStaticRp, ONE_SECOND};
use embassy_executor::Spawner;
use embassy_net::Stack;

async fn test_two_clock_syncs(stack: &'static Stack<'static>, spawner: Spawner) {
    static CLOCK_SYNC0_STATIC: ClockSyncStaticRp = ClockSyncRp::new_static();
    static CLOCK_SYNC1_STATIC: ClockSyncStaticRp = ClockSyncRp::new_static();

    let _clock_sync0 = ClockSyncRp::new(&CLOCK_SYNC0_STATIC, stack, 0, Some(ONE_SECOND), spawner);
    let _clock_sync1 = ClockSyncRp::new(&CLOCK_SYNC1_STATIC, stack, 0, Some(ONE_SECOND), spawner);
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
