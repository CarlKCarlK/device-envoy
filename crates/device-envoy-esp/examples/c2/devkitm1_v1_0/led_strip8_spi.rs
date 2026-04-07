//! LED strip example: 8 NeoPixel-style (WS2812) LEDs via SPI MOSI.
//!
//! Wiring:
//! - `esp_gdma_family` (ESP32-C3/C6/S3): data-in on GPIO10
//! - `esp_pdma_family` (ESP32/S2): data-in on GPIO4

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_esp::{
    Result, init_and_start, led_strip,
    led_strip::{Current, Frame1d, LedStrip as _, colors},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
const STRIP8_PIN_NUM: u8 = 10;
#[cfg(esp_pdma_family)] // original ESP32 & s2
const STRIP8_PIN_NUM: u8 = 4;
const FRAME_DURATION: Duration = Duration::from_millis(350);

#[cfg(esp_gdma_family)] // C6, S3, etc
led_strip! {
    LedStripLen8Spi {
        pin: GPIO10,
        len: 8,
        max_current: Current::Milliamps(180),
        engine: Engine::Spi,
        max_frames: 2,
    }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
led_strip! {
    LedStripLen8Spi {
        pin: GPIO4,
        len: 8,
        max_current: Current::Milliamps(180),
        engine: Engine::Spi,
        max_frames: 2,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!(
        "led_strip_len8_spi starting on GPIO{} via SPI2",
        STRIP8_PIN_NUM
    );

    #[cfg(esp_gdma_family)]
    let led_strip_len8_spi = LedStripLen8Spi::new(p.GPIO10, p.SPI2, spawner)?;
    #[cfg(esp_pdma_family)]
    let led_strip_len8_spi = LedStripLen8Spi::new(p.GPIO4, p.SPI2, spawner)?;
    let frame0 = Frame1d([
        colors::BLUE,
        colors::BLACK,
        colors::BLUE,
        colors::BLACK,
        colors::BLUE,
        colors::BLACK,
        colors::BLUE,
        colors::BLACK,
    ]);
    let frame1 = Frame1d([
        colors::BLACK,
        colors::ORANGE,
        colors::BLACK,
        colors::ORANGE,
        colors::BLACK,
        colors::ORANGE,
        colors::BLACK,
        colors::ORANGE,
    ]);
    led_strip_len8_spi.animate([(frame0, FRAME_DURATION), (frame1, FRAME_DURATION)]);

    core::future::pending().await
}
