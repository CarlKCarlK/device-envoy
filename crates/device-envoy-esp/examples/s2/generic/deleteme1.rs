// @generated examples/templates/deleteme1.rs.j2 by cargo xtask generate-board-examples.
#![allow(missing_docs)]
//! deleteme1: 8-pixel strip with blue/gray swap on button press.
//!
//! Wiring:
//! - NeoPixel-style (WS2812) strip data input -> GPIO10
//! - Button input -> GPIO0 to GND (`PressedTo::Ground`)
//! - Shared GND between board, strip, and button
//!
//! Press the button to swap which color (blue or gray) is on even/odd pixels.
//!
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    Result,
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start, led_strip,
    led_strip::{Current, Frame1d, LedStrip as _, colors},
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    LedStripLen8 {
        pin: GPIO10,
        len: 8,
        max_current: Current::Milliamps(250),
    }
}

fn write_blue_gray_pattern(led_strip_len8: &LedStripLen8, blue_on_even_pixels: bool) {
    let mut frame1d = Frame1d::new();
    for pixel_index in 0..frame1d.len() {
        let even_pixel = pixel_index % 2 == 0;
        frame1d[pixel_index] = if even_pixel == blue_on_even_pixels {
            colors::BLUE
        } else {
            colors::GRAY
        };
    }
    led_strip_len8.write_frame(frame1d);
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let led_strip_len8 = LedStripLen8::new(p.GPIO10, rmt80.channel0, spawner)?;
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);

    let mut blue_on_even_pixels = true;
    write_blue_gray_pattern(led_strip_len8, blue_on_even_pixels);
    info!("deleteme1 ready: press button to swap blue/gray positions");

    loop {
        button.wait_for_press().await;
        blue_on_even_pixels = !blue_on_even_pixels;
        write_blue_gray_pattern(led_strip_len8, blue_on_even_pixels);
        info!("swapped pattern");
    }
}
