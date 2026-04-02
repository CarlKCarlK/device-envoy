//! SOS blinky.
//!
//! Wiring:
//! - ESP32-C3: external plain LED on GPIO2 (active-high)
//! - RMT-capable chips (except ESP32-C3): built-in NeoPixel-style (WS2812) LED on GPIO8/0
//! - Non-RMT chips (for example ESP32-C2): NeoPixel-style (WS2812) LED via SPI on GPIO0
//!
//! Non-RMT chips usually need an external NeoPixel-style (WS2812) LED/strip on GPIO0.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

#[cfg(feature = "esp32c3")]
use device_envoy_esp::{
    init_and_start, led,
    led::{Led as _, LedLevel, OnLevel},
    Result,
};

#[cfg(not(feature = "esp32c3"))]
#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
#[cfg(not(feature = "esp32c3"))]
use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Frame1d, LedStrip as _},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(feature = "esp32c3")]
const LED_PIN_NUM: u8 = 2;
#[cfg(all(not(feature = "esp32c3"), esp_has_rmt, esp_gdma_family))] // C6, S3, etc
const LED_PIN_NUM: u8 = 8;
#[cfg(all(not(feature = "esp32c3"), not(all(esp_has_rmt, esp_gdma_family))))] // original ESP32 & s2, plus non-RMT chips
const LED_PIN_NUM: u8 = 0;

// TODO00 After examples/blinky.rs is stable on current boards, confirm whether esp32c3 should keep external plain LED as the canonical path or regain an onboard NeoPixel-style (WS2812) path.

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

#[cfg(feature = "esp32c3")]
const SOS: [(LedLevel, Duration); 18] = [
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

#[cfg(not(feature = "esp32c3"))]
const ON_COLOR: Frame1d<1> = Frame1d([colors::WHITE]);
#[cfg(not(feature = "esp32c3"))]
const OFF_COLOR: Frame1d<1> = Frame1d([colors::BLACK]);

#[cfg(not(feature = "esp32c3"))]
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

#[cfg(feature = "esp32c3")]
led! {
    SosLed {
        pin: GPIO2,
        max_steps: 20,
    }
}

#[cfg(all(not(feature = "esp32c3"), esp_has_rmt, esp_gdma_family))] // C6, S3, etc
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(all(not(feature = "esp32c3"), not(esp_has_rmt)))] // non-RMT chips (currently ESP32-C2)
led_strip! {
    SosStrip {
        pin: GPIO0,
        len: 1,
        engine: Engine::Spi,
        max_frames: 20,
    }
}

#[cfg(all(not(feature = "esp32c3"), esp_has_rmt, esp_pdma_family))] // original ESP32 & s2
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
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<core::convert::Infallible> {
    #[cfg(feature = "esp32c3")]
    init_and_start!(p);
    #[cfg(all(not(feature = "esp32c3"), esp_has_rmt))]
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    #[cfg(all(not(feature = "esp32c3"), not(esp_has_rmt)))]
    init_and_start!(p);

    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("SOS blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(feature = "esp32c3")]
    {
        let sos_led = SosLed::new(p.GPIO2, OnLevel::High, spawner)?;
        sos_led.animate(SOS);
    }

    #[cfg(all(not(feature = "esp32c3"), esp_has_rmt, esp_gdma_family))]
    {
        let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel0, spawner)?;
        sos_strip.animate(&SOS);
    }

    #[cfg(all(not(feature = "esp32c3"), esp_has_rmt, esp_pdma_family))]
    {
        let sos_strip = SosStrip::new(p.GPIO0, rmt80.channel0, spawner)?;
        sos_strip.animate(&SOS);
    }

    #[cfg(all(not(feature = "esp32c3"), not(esp_has_rmt)))]
    {
        let sos_strip = SosStrip::new(p.GPIO0, p.SPI2, spawner)?;
        sos_strip.animate(&SOS);
    }

    core::future::pending().await
}
