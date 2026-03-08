#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, lcd_text};
use device_envoy_esp::lcd_text::LcdText as _;

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

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);

    let lcd_text_simple = LcdTextSimple::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;

    lcd_text_simple.write_text("This line is definitely longer than sixteen\nAnd this one too");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Unicode: cafe\u{301} ☕\nnaive — piñata");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Line 1\nLine 2\nLine 3\nLine 4");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("");
    Timer::after(Duration::from_secs(1)).await;

    lcd_text_simple.write_text("Hello from\ndevice-envoy!");

    core::future::pending().await
}
