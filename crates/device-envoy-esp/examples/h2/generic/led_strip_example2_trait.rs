//! Wiring:
//! - 12x8 NeoPixel-style (WS2812) panel data input -> GPIO2
//!
#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    Result, init_and_start, led_strip,
    led_strip::{Current, Frame1d, Gamma, LedStrip, colors},
};

esp_bootloader_esp_idf::esp_app_desc!();

led_strip! {
    LedStripLen96 {
        pin: GPIO2,
        len: 96,
        max_current: Current::Milliamps(1000),
        gamma: Gamma::Linear,
        max_frames: 3,
    }
}

fn animate_rgb_cycle<const N: usize>(led_strip: &impl LedStrip<N>) {
    // Create a sequence of frames and durations and then animate them (looping, until replaced).
    let frame_duration = Duration::from_millis(300);
    led_strip.animate([
        (Frame1d::filled(colors::RED), frame_duration),
        (Frame1d::filled(colors::GREEN), frame_duration),
        (Frame1d::filled(colors::BLUE), frame_duration),
    ]);
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("LED strip trait example 2: RGB animation on GPIO2");

    let led_strip_len96 = LedStripLen96::new(p.GPIO2, rmt80.channel0, spawner)?;
    animate_rgb_cycle(led_strip_len96);

    core::future::pending().await
}
