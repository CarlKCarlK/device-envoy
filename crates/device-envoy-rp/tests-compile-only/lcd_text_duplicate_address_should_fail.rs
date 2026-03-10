#![allow(missing_docs)]
//! Compile-only negative test: duplicate LCD I2C addresses in one i2cs! group
//! must fail at compile time.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::i2cs;
use embassy_executor::Spawner;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTextsDuplicateAddress {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x27 },
    }
}

async fn test_duplicate_address(_p: embassy_rp::Peripherals, _spawner: Spawner) {}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
