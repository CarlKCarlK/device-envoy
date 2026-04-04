//! Embedded compile-fail test target for duplicate LCD I2C addresses.
//!
//! This intentionally repeats one address in an i2cs! group and should fail
//! at compile time.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{i2cs, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32c3")]
i2cs! {
    i2c: I2C0,
    sda_pin: GPIO4,
    scl_pin: GPIO5,
    I2csDuplicateAddressCompileFail {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x27 },
    }
}

#[cfg(not(feature = "esp32c3"))]
i2cs! {
    i2c: I2C0,
    sda_pin: GPIO16,
    scl_pin: GPIO17,
    I2csDuplicateAddressCompileFail {
        LcdTextA { width: 16, height: 2, address: 0x27 },
        LcdTextB { width: 20, height: 4, address: 0x27 },
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
    let (_lcd_text_a, _lcd_text_b) =
        I2csDuplicateAddressCompileFail::new(p.I2C0, p.GPIO4, p.GPIO5, spawner)?;
    #[cfg(not(feature = "esp32c3"))]
    let (_lcd_text_a, _lcd_text_b) =
        I2csDuplicateAddressCompileFail::new(p.I2C0, p.GPIO16, p.GPIO17, spawner)?;

    core::future::pending().await
}
