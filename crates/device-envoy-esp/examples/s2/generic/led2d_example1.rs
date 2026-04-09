//! Wiring:
//! - 12x4 NeoPixel-style (WS2812) panel data input -> GPIO17
//!
#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_core::led2d::Led2d as _;
use device_envoy_esp::{
    Result, init_and_start,
    led_strip::Current,
    led2d,
    led2d::{Led2dFont, layout::LedLayout},
};
use smart_leds::RGB8;

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();

led2d! {
    Led12x4 {
        pin: GPIO17,
        len: 48,
        led_layout: LED_LAYOUT_12X4,
        max_current: Current::Milliamps(250),
        font: Led2dFont::Font3x4Trim,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("LED 2D trait example 1: Write text on a 12x4 panel via GPIO17");

    let led12x4 = Led12x4::new(p.GPIO17, rmt80.channel0, spawner)?;

    let colors = [
        RGB8::new(0, 255, 255),
        RGB8::new(255, 0, 0),
        RGB8::new(255, 255, 0),
    ];
    led12x4.write_text("Rust", &colors);

    pending().await
}
