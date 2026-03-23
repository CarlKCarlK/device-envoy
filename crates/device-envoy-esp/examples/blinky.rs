//! SOS blinky on the built-in LED.
//!
//! Wiring:
//! - RMT-capable chips: built-in NeoPixel-style (WS2812) LED on GPIO8/0
//! - Non-RMT chips (for example ESP32-C2): NeoPixel-style (WS2812) LED via SPI on GPIO0
//!
//! Non-RMT chips usually need an external NeoPixel-style (WS2812) LED/strip on GPIO0.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_esp::{
    led_strip,
    led_strip::{colors, Frame1d, LedStrip as _},
    init_and_start, Result,
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

#[cfg(all(esp_has_rmt, esp_gdma_family))] // C6, S3, etc
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(not(esp_has_rmt))] // non-RMT chips (currently ESP32-C2)
led_strip! {
    SosStrip {
        pin: GPIO0,
        len: 1,
        engine: Engine::Spi,
        max_frames: 20,
    }
}

#[cfg(all(esp_has_rmt, esp_pdma_family))] // original ESP32 & s2
led_strip! {
    SosStrip {
        pin: GPIO0,
        len: 1,
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
    #[cfg(not(esp_has_rmt))] // non-RMT chips (currently ESP32-C2)
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("SOS blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(all(esp_has_rmt, esp_gdma_family))] // C6, S3, etc
    let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel0, spawner)?;
    #[cfg(all(esp_has_rmt, esp_pdma_family))] // original ESP32 & s2
    let sos_strip = SosStrip::new(p.GPIO0, rmt80.channel0, spawner)?;
    #[cfg(not(esp_has_rmt))] // non-RMT chips (currently ESP32-C2)
    let sos_strip = SosStrip::new(p.GPIO0, p.SPI2, spawner)?;
    sos_strip.animate(&SOS);

    core::future::pending().await
}
