#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::{convert::Infallible, future, panic};
use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Frame1d, LedStrip as _},
    Result,
};
use embassy_executor::Spawner;
use esp_backtrace as _;

#[cfg(any(feature = "esp32", feature = "esp32h2"))]
led_strip! {
    LedStripLen8 {
        pin: GPIO2,
        len: 8,
    }
}

#[cfg(not(any(feature = "esp32", feature = "esp32h2")))]
led_strip! {
    LedStripLen8 {
        pin: GPIO10,
        len: 8,
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

// Nice trick: Two "mains" lets us use Results.
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    #[cfg(any(feature = "esp32", feature = "esp32h2"))]
    let led_strip_len8 = LedStripLen8::new(p.GPIO2, rmt80.channel0, spawner)?;
    #[cfg(not(any(feature = "esp32", feature = "esp32h2")))]
    let led_strip_len8 = LedStripLen8::new(p.GPIO10, rmt80.channel0, spawner)?;

    // Fill an array of pixels with alternating blue and gray colors
    let mut frame1d = Frame1d::new(); // just an owned array of RGB pixels
    let palette = [colors::BLUE, colors::LIGHT_GRAY];
    for pixel_index in 0..frame1d.len() {
        frame1d[pixel_index] = palette[pixel_index % 2];
    }

    // Write the frame to the LED strip. Will stay until replaced.
    led_strip_len8.write_frame(frame1d);
    future::pending().await // run forever
}
