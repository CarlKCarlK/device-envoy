#![allow(missing_docs)]
//! Compile-only negative test: constructing a single-device and then the group
//! from the same owned hardware resources must fail.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::Result;
use device_envoy_rp::i2cs;
use embassy_executor::Spawner;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTexts0 {
        LcdTextSimple { width: 16, height: 2, address: 0x27 },
    }
}

async fn test_conflicting_construction(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let _lcd_text_simple = LcdTextSimple::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
    let (_lcd_text_simple_again,) = LcdTexts0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
