#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_core::lcd_text::LcdText;
use device_envoy_esp::{init_and_start, lcd_text, Result};

esp_bootloader_esp_idf::esp_app_desc!();

lcd_text! {
    i2c: I2C0,
    sda_pin: GPIO16,
    scl_pin: GPIO17,
    LcdTextSimple {
        width: 16,
        height: 2,
        address: 0x27
    }
}

fn write_message<const W: usize, const H: usize>(lcd_text: &impl LcdText<W, H>) {
    lcd_text.write_text("Hello from\ndevice-envoy!");
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);

    let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;
    write_message(lcd_text_simple);

    core::future::pending().await
}
