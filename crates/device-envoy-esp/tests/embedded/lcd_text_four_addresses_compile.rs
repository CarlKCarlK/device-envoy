//! Embedded compile-only test target for four LCD text addresses on one I2C bus.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::lcd_text::LcdText as _;
use device_envoy_esp::{i2cs, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32c3")]
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

#[cfg(not(feature = "esp32c3"))]
i2cs! {
    i2c: I2C0,
    sda_pin: GPIO16,
    scl_pin: GPIO17,
    I2cs4 {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x26 },
        LcdTextC { width: 16, height: 2, address: 0x25 },
        LcdTextD { width: 20, height: 4, address: 0x24 },
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p);

    #[cfg(feature = "esp32c3")]
    let (lcd_text_a, lcd_text_b, lcd_text_c, lcd_text_d) =
        I2cs4::new(p.I2C0, p.GPIO4, p.GPIO5, spawner)?;
    #[cfg(not(feature = "esp32c3"))]
    let (lcd_text_a, lcd_text_b, lcd_text_c, lcd_text_d) =
        I2cs4::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;

    lcd_text_a.write_text("A");
    lcd_text_b.write_text("B");
    lcd_text_c.write_text("C");
    lcd_text_d.write_text("D");

    core::future::pending().await
}
