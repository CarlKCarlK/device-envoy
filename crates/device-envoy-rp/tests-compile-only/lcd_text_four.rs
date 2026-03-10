#![allow(missing_docs)]
//! Compile-only verification for four LCD text devices sharing one I2C resource.

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
    LcdTexts4 {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x26 },
        LcdTextC { width: 16, height: 2, address: 0x25 },
        LcdTextD { width: 20, height: 4, address: 0x24 },
    }
}

async fn test_four_lcd_texts(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (lcd_text_a, lcd_text_b, lcd_text_c, lcd_text_d) =
        LcdTexts4::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;

    lcd_text_a.write_text("A");
    lcd_text_b.write_text("B");
    lcd_text_c.write_text("C");
    lcd_text_d.write_text("D");
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
