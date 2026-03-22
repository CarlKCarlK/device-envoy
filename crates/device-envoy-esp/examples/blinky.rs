//! SOS blinky on the built-in LED.
//!
//! Wiring:
//! - RMT-capable chips: built-in NeoPixel-style (WS2812) LED on GPIO8/0
//! - Non-RMT chips (for example ESP32-C2): digital LED fallback on GPIO0
//!
//! No external wiring needed.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{init_and_start, Result};
#[cfg(not(esp_has_rmt))]
use device_envoy_esp::{
    led,
    led::{Led as _, LedLevel, OnLevel},
};
#[cfg(esp_has_rmt)]
use device_envoy_esp::{
    led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(all(esp_has_rmt, esp_gdma_family))] // C6, S3, etc
const LED_PIN_NUM: u8 = 8;
#[cfg(not(all(esp_has_rmt, esp_gdma_family)))] // original ESP32 & s2, plus non-RMT chips
const LED_PIN_NUM: u8 = 0;

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
#[cfg(esp_has_rmt)]
const ON_COLOR: Frame1d<1> = Frame1d([colors::WHITE]);
#[cfg(esp_has_rmt)]
const OFF_COLOR: Frame1d<1> = Frame1d([colors::BLACK]);

// SOS: . . .   - - -   . . .
#[cfg(esp_has_rmt)]
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

#[cfg(all(esp_has_rmt, esp_gdma_family))] // C6, S3, etc
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_current: Current::Milliamps(10),
        max_frames: 20,
    }
}

#[cfg(not(esp_has_rmt))]
led! {
    SosStrip {
        pin: GPIO0,
        max_steps: 20,
    }
}

#[cfg(not(esp_has_rmt))]
const SOS_LEVELS: [(LedLevel, Duration); 18] = [
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, LETTER_GAP),
    (LedLevel::On, DASH_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DASH_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DASH_DURATION),
    (LedLevel::Off, LETTER_GAP),
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, SYMBOL_GAP),
    (LedLevel::On, DOT_DURATION),
    (LedLevel::Off, WORD_GAP),
];

#[cfg(all(esp_has_rmt, esp_pdma_family))] // original ESP32 & s2
led_strip! {
    SosStrip {
        pin: GPIO0,
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
    #[cfg(esp_has_rmt)]
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    #[cfg(not(esp_has_rmt))]
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("SOS blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(all(esp_has_rmt, esp_gdma_family))] // C6, S3, etc
    let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel0, spawner)?;
    #[cfg(all(esp_has_rmt, esp_pdma_family))] // original ESP32 & s2
    let sos_strip = SosStrip::new(p.GPIO0, rmt80.channel0, spawner)?;
    #[cfg(esp_has_rmt)]
    sos_strip.animate(&SOS);
    #[cfg(not(esp_has_rmt))]
    {
        let sos_strip = SosStrip::new(p.GPIO0, OnLevel::High, spawner)?;
        sos_strip.animate(&SOS_LEVELS);
    }

    core::future::pending().await
}
