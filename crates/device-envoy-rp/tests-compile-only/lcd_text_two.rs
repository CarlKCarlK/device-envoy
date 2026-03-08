#![allow(missing_docs)]
//! Compile-only verification for two LCD text devices sharing one I2C resource.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::i2cs;
use embassy_executor::Spawner;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTexts0 {
        LcdText16x2 { width: 16, height: 2, address: 0x27 },
        LcdText20x4 { width: 20, height: 4, address: 0x3F },
    }
}

async fn test_two_lcd_texts(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (lcd_text16x2, lcd_text20x4) = LcdTexts0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
    lcd_text16x2.write_text("16x2 ready");
    lcd_text20x4.write_text("20x4 ready");
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
