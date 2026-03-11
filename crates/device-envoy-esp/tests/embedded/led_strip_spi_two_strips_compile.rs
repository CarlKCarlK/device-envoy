//! Embedded compile-only test target for SPI-backed LED strips.
//!
//! On ESP32-S3 this validates two SPI-backed strips using distinct SPI peripherals.
//! On ESP32-C6 this validates single-strip SPI construction (only one SPI peripheral
//! is exposed to this abstraction path).

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    LedStripSpiA {
        len: 8,
        max_current: Current::Milliamps(120),
        max_frames: 2,
        engine: device_envoy_esp::led_strip::Engine::Spi,
    }
}

led_strip! {
    LedStripSpiB {
        len: 1,
        max_current: Current::Milliamps(10),
        engine: device_envoy_esp::led_strip::Engine::Spi,
        max_frames: 2,
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
    init_and_start!(p, rmt80, rmt_mode::Blocking);

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

    #[cfg(target_arch = "xtensa")]
    {
        let led_strip_spi_b = LedStripSpiB::new(p.GPIO11, p.SPI3, spawner)?;
        led_strip_spi_b.write_frame(Frame1d([colors::RED]));
    }

    let _ = rmt80;
    core::future::pending().await
}
