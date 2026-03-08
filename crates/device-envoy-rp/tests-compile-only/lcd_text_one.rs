#![allow(missing_docs)]
//! Compile-only verification for a single LCD text device using `i2cs!`.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::i2cs;
use device_envoy_rp::lcd_text::LcdText as _;
use embassy_executor::Spawner;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTexts0 {
        LcdTextSimple { width: 16, height: 2, address: 0x27 },
    }
}

async fn test_single_lcd_text(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
    lcd_text_simple.write_text("compile-only\nsingle");
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
