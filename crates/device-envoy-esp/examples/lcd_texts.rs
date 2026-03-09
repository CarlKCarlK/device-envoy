#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;

use device_envoy_esp::lcd_text::LcdText as _;
use device_envoy_esp::{i2cs, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

i2cs! {
    i2c: I2C0,
    sda_pin: GPIO16,
    scl_pin: GPIO17,
    LcdTexts0 {
        LcdText16x2 { width: 16, height: 2, address: 0x27 },
        LcdText20x4 { width: 20, height: 4, address: 0x26 },
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

    let (lcd_text16x2, lcd_text20x4) = LcdTexts0::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;

    loop {
        lcd_text16x2.write_text("LCD #1\n16x2");
        lcd_text20x4.write_text("LCD #2\n20x4\nshared i2c\naddress 0x3F");
        Timer::after(Duration::from_secs(1)).await;

        lcd_text16x2.write_text("Tick");
        lcd_text20x4.write_text("Tock");
        Timer::after(Duration::from_secs(1)).await;
    }
}
