//! Animated text on a 12x8 panel (displayed as rotated 8x12) on GPIO18.
//!
//! Standard demo pin map in this repo:
//! - GPIO8 (default): built-in single smart-LED demo (`blinky_smart_led.rs`)
//! - GPIO10: external 8-pixel strip demo (`led_strip_len8.rs`)
//! - GPIO18: 12x8 panel `Go`/`\\nGo` demo (this file)

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led2d,
    led2d::Led2d as _,
    led2d::{layout::LedLayout, Frame2d, Led2dFont},
    led_strip::{colors, Current, Gamma},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
const LED_LAYOUT_8X12_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();

led2d! {
    Led12x8Animated {
        pin: GPIO18,
        len: 96,
        led_layout: LED_LAYOUT_8X12_ROTATED,
        max_current: Current::Milliamps(300),
        font: Led2dFont::Font4x6Trim,
        gamma: Gamma::Linear,
        max_frames: 2,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("LED 2D Example: Animated text on rotated 12x8 panel via GPIO18");

    let led12x8_animated = Led12x8Animated::new(p.GPIO18, rmt80.channel0, spawner)?;

    let mut frame_0 = Frame2d::new();
    led12x8_animated.write_text_to_frame("Go", &[], &mut frame_0);

    let mut frame_1 = Frame2d::new();
    led12x8_animated.write_text_to_frame("\nGo", &[colors::HOT_PINK, colors::LIME], &mut frame_1);

    let frame_duration = Duration::from_secs(1);
    led12x8_animated.animate([(frame_0, frame_duration), (frame_1, frame_duration)]);

    core::future::pending().await
}
