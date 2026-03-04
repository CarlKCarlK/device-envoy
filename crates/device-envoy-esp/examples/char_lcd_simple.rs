//! Minimal character LCD test for HD44780 I2C backpacks.
//!
//! Wiring:
//! - SDA: GPIO1
//! - SCL: GPIO2
//! - LCD VCC: 3.3V (recommended)
//! - LCD GND: GND
//!
//! This example intentionally avoids Wi-Fi/clock setup so LCD bring-up is isolated.
// TODO00000 This isn't working and char_lcd is untested.
// TODO00000 Need to review the construction API for char_lcd.

#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    char_lcd::{CharLcd, CharLcdStatic},
    init_and_start, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("Starting char_lcd_simple on GPIO1/2");

    let i2c = esp_hal::i2c::master::I2c::new(p.I2C0, esp_hal::i2c::master::Config::default())
        .expect("I2C0 config should be valid")
        .with_sda(p.GPIO1)
        .with_scl(p.GPIO2);

    static CHAR_LCD_STATIC: CharLcdStatic = CharLcd::new_static();
    let char_lcd = CharLcd::new(&CHAR_LCD_STATIC, i2c, spawner)?;

    char_lcd
        .write_text(
            "Hello\nDevice Envoy"
                .try_into()
                .expect("initial text must fit"),
            0,
        )
        .await;

    loop {
        char_lcd
            .write_text(
                "LCD test\ncounting..."
                    .try_into()
                    .expect("loop text must fit"),
                1500,
            )
            .await;
        Timer::after(Duration::from_millis(1500)).await;

        char_lcd
            .write_text(
                "If blank,\ncheck addr"
                    .try_into()
                    .expect("loop text must fit"),
                1500,
            )
            .await;
        Timer::after(Duration::from_millis(1500)).await;
    }
}
