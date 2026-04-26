//! Embedded compile-only test target for SPI-backed LED strips.
//!
//! On ESP32-S3 this validates two SPI-backed strips using distinct SPI peripherals.
//! On ESP32-C6 this validates single-strip SPI construction (only one SPI peripheral
//! is exposed to this abstraction path).

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{Current, Frame1d, LedStrip as _, colors},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32")]
led_strip! {
    LedStripSpiA {
        engine: device_envoy_esp::led_strip::Engine::Spi,
        len: 8,
        pin: GPIO0,
        max_frames: 2,
        max_current: Current::Milliamps(120),
    }
}

#[cfg(not(feature = "esp32"))]
led_strip! {
    LedStripSpiA {
        engine: device_envoy_esp::led_strip::Engine::Spi,
        len: 8,
        pin: GPIO10,
        max_frames: 2,
        max_current: Current::Milliamps(120),
    }
}

#[cfg(all(target_arch = "xtensa", not(feature = "esp32")))]
led_strip! {
    LedStripSpiB {
        max_frames: 2,
        engine: device_envoy_esp::led_strip::Engine::Spi,
        len: 1,
        pin: GPIO11,
    }
}

#[cfg(not(target_arch = "xtensa"))]
led_strip! {
    LedStripSpiB {
        max_frames: 2,
        engine: device_envoy_esp::led_strip::Engine::Spi,
        len: 1,
        pin: GPIO6,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    #[cfg(esp_has_rmt)]
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    #[cfg(not(esp_has_rmt))]
    init_and_start!(p);

    #[cfg(feature = "esp32")]
    let led_strip_spi_a = LedStripSpiA::new(p.GPIO0, p.SPI2, spawner)?;
    #[cfg(not(feature = "esp32"))]
    let led_strip_spi_a = LedStripSpiA::new(p.GPIO10, p.SPI2, spawner)?;
    led_strip_spi_a.write_frame(Frame1d([
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
    ]));

    #[cfg(all(target_arch = "xtensa", not(feature = "esp32")))]
    {
        let led_strip_spi_b = LedStripSpiB::new(p.GPIO11, p.SPI3, spawner)?;
        led_strip_spi_b.write_frame(Frame1d([colors::RED]));
    }

    #[cfg(esp_has_rmt)]
    let _ = rmt80;
    core::future::pending().await
}
