#![allow(missing_docs)]
//! Compile-only negative test: `Name: { ... }` form is rejected for `ir!`.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::ir;
use embassy_executor::Spawner;

ir! {
    IrOldStyle: { pio: PIO0, pin: PIN_15 }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
