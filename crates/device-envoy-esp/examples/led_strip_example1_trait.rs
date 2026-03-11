#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    LedStripSimple {
        pin: GPIO10,
        len: 8,
        max_current: Current::Milliamps(250),
    }
}

fn write_alternating_blue_gray<const N: usize>(led_strip: &impl LedStrip<N>) {
    // Create and write a frame with alternating blue and gray pixels.
    let mut frame = Frame1d::new();
    for pixel_index in 0..N {
        // Directly index into the frame buffer.
        frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
    }
    // Display the frame on the LED strip (until replaced).
    led_strip.write_frame(frame);
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("LED strip trait example 1: alternating blue/gray frame on GPIO10");

    let led_strip_simple = LedStripSimple::new(p.GPIO10, rmt80.channel0, spawner)?;
    write_alternating_blue_gray(led_strip_simple);

    core::future::pending().await
}
