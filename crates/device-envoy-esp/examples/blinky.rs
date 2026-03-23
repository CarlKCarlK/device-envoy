//! SOS blinky using `led_strip!` on the built-in WS2812 LED.
//!
//! Wiring:
//! - ESP32-C6-DevKitC-1: built-in LED on GPIO8
//! - ESP32-S3-DevKitC-1: built-in LED on GPIO38
//!
//! No external wiring needed.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32c6")]
const LED_PIN_NUM: u8 = 8;
#[cfg(feature = "esp32s3")]
const LED_PIN_NUM: u8 = 38;

// Morse timing (in milliseconds).
const DOT_MS: u64 = 200;
const DASH_MS: u64 = DOT_MS * 3;
const SYMBOL_GAP_MS: u64 = DOT_MS;
const LETTER_GAP_MS: u64 = DOT_MS * 3;
const WORD_GAP_MS: u64 = DOT_MS * 7;

const DOT_DURATION: Duration = Duration::from_millis(DOT_MS);
const DASH_DURATION: Duration = Duration::from_millis(DASH_MS);
const SYMBOL_GAP: Duration = Duration::from_millis(SYMBOL_GAP_MS);
const LETTER_GAP: Duration = Duration::from_millis(LETTER_GAP_MS);
const WORD_GAP: Duration = Duration::from_millis(WORD_GAP_MS);

// Use a dim white so the LED is visible but not blinding.
const ON_COLOR: Frame1d<1> = Frame1d([colors::WHITE]);
const OFF_COLOR: Frame1d<1> = Frame1d([colors::BLACK]);

// SOS: . . .   - - -   . . .
const SOS: [(Frame1d<1>, Duration); 18] = [
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, LETTER_GAP),
    (ON_COLOR, DASH_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DASH_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DASH_DURATION),
    (OFF_COLOR, LETTER_GAP),
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, SYMBOL_GAP),
    (ON_COLOR, DOT_DURATION),
    (OFF_COLOR, WORD_GAP),
];

#[cfg(feature = "esp32c6")]
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_current: Current::Milliamps(10),
        max_frames: 20,
    }
}

#[cfg(feature = "esp32s3")]
led_strip! {
    SosStrip {
        pin: GPIO38,
        len: 1,
        max_current: Current::Milliamps(10),
        max_frames: 20,
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

    info!("SOS blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(feature = "esp32c6")]
    let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel0, spawner)?;
    #[cfg(feature = "esp32s3")]
    let sos_strip = SosStrip::new(p.GPIO38, rmt80.channel0, spawner)?;
    sos_strip.animate(&SOS);

    core::future::pending().await
}
