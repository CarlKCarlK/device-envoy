//! Embedded compile-only check that the code `lcd_text!` and `i2cs!` generate builds
//! cleanly in a user crate that denies warnings and missing docs. Separate from
//! `generated_code_lints_compile` so it runs on every chip, including those without RMT
//! or LEDC.

#![no_std]
#![no_main]
#![deny(warnings, missing_docs)]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, lcd_text, lcd_text::LcdText as _};

esp_bootloader_esp_idf::esp_app_desc!();

lcd_text! {
    /// A public LCD.
    pub PublicLcd { i2c: I2C0, sda_pin: GPIO4, scl_pin: GPIO5, width: 16, height: 2, address: 0x27 }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);
    let public_lcd = PublicLcd::new(p.GPIO4, p.GPIO5, p.I2C0, spawner)?;
    public_lcd.write_text("Hello");
    core::future::pending().await
}
