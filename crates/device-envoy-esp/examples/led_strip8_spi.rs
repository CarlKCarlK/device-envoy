//! LED strip example: 8 NeoPixel-style (WS2812) LEDs on GPIO10 via SPI MOSI.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;
#[allow(unused_imports)]
use device_envoy_esp32::led_strip::Engine;

use device_envoy_esp32::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d},
};

esp_bootloader_esp_idf::esp_app_desc!();

const STRIP8_PIN_NUM: u8 = 10;
const FRAME_DURATION: Duration = Duration::from_millis(350);

led_strip! {
    LedStrip8Spi {
        len: 8,
        max_current: Current::Milliamps(180),
        engine: Engine::Spi,
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

async fn inner_main(spawner: Spawner) -> device_envoy_esp32::Result<core::convert::Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("led_strip8_spi starting on GPIO{} via SPI2", STRIP8_PIN_NUM);

    let led_strip8_spi = LedStrip8Spi::new(p.GPIO10, p.SPI2, spawner)?;
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
    led_strip8_spi.animate([(frame0, FRAME_DURATION), (frame1, FRAME_DURATION)])?;

    core::future::pending().await
}
