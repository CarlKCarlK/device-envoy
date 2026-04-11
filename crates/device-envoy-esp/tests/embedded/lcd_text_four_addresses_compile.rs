//! Embedded compile-only test target for four LCD text addresses on one I2C bus.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::lcd_text::LcdText as _;
use device_envoy_esp::{i2cs, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

i2cs! {
    i2c: I2C0,
    sda_pin: GPIO4,
    scl_pin: GPIO5,
    I2cs4 {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x26 },
        LcdTextC { width: 16, height: 2, address: 0x25 },
        LcdTextD { width: 20, height: 4, address: 0x24 },
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);

    let (lcd_text_a, lcd_text_b, lcd_text_c, lcd_text_d) =
        I2cs4::new(p.I2C0, p.GPIO4, p.GPIO5, spawner)?;

    lcd_text_a.write_text("A");
    lcd_text_b.write_text("B");
    lcd_text_c.write_text("C");
    lcd_text_d.write_text("D");

    core::future::pending().await
}
