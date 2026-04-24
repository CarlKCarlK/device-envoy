//! Wiring:
//! - 8-pixel NeoPixel-style (WS2812) strip data input -> GPIO2
//!
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    LedStripLen8 {
        pin: GPIO2,
        len: 8,
        max_current: Current::Milliamps(250),
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

    info!("LED strip trait example 1: alternating blue/gray frame");

    let led_strip_len8 = LedStripLen8::new(p.GPIO2, rmt80.channel0, spawner)?;

    // Create and write a frame with alternating blue and gray pixels.
    let mut frame = Frame1d::new();
    for pixel_index in 0..8 {
        // Directly index into the frame buffer.
        frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
    }
    // Display the frame on the LED strip (until replaced).
    led_strip_len8.write_frame(frame);

    pending().await
}
