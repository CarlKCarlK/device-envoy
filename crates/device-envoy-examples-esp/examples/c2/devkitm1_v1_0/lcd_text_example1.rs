//! Wiring:
//! - I2C LCD SDA -> GPIO4
//! - I2C LCD SCL -> GPIO5
//!
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_core::lcd_text::LcdText as _;
use device_envoy_esp::{Result, init_and_start, lcd_text};

esp_bootloader_esp_idf::esp_app_desc!();

lcd_text! {
    i2c: I2C0,
    sda_pin: GPIO4,
    scl_pin: GPIO5,
    LcdTextSimple {
        width: 16,
        height: 2,
        address: 0x27
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);

    let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.GPIO4, p.GPIO5, spawner)?;
    lcd_text_simple.write_text("Hello from\ndevice-envoy!");

    pending().await
}
