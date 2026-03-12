//! LED strip example: 8 NeoPixel-style (WS2812) LEDs on GPIO10 (blue/gray pattern)
//! plus SOS blink on the built-in WS2812 LED, using two RMT TX channels.
//!
//! Standard demo pin map in this repo:
//! - GPIO8/C6 or GPIO48/S3: built-in single LED strip demo (`blinky.rs`)
//! - GPIO10: external 8-pixel strip demo (this file)
//! - GPIO18: 12x8 panel `Go`/`\\nGo` demo (`led2d.rs`)
//!
//! Wiring:
//! - GPIO10: data-in of an 8-pixel NeoPixel-style (WS2812) strip
//! - GPIO8 (C6) or GPIO48 (S3): on-board NeoPixel-style (WS2812) RGB LED
//!
//! Both strips share one RMT hub with explicit TX channel ownership.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Current, Frame1d, LedStrip as _},
};

esp_bootloader_esp_idf::esp_app_desc!();

// todo0 delete
// const STRIP8_PIN_NUM: u8 = 10;
// #[cfg(target_arch = "riscv32")]
// const BUILTIN_LED_PIN_NUM: u8 = 8;
// #[cfg(target_arch = "xtensa")]
// const BUILTIN_LED_PIN_NUM: u8 = 48;

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

led_strip! {
    LedStrip8 {
        pin: GPIO10,
        len: 8,
        max_current: Current::Milliamps(200),
        max_frames: 2,
    }
}
#[cfg(target_arch = "riscv32")]
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_current: Current::Milliamps(10),
        max_frames: 20,
    }
}
#[cfg(target_arch = "xtensa")]
led_strip! {
    SosStrip {
        pin: GPIO48,
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

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: init_and_start::rmt_mode::Blocking);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    //info!("LED strip 8 starting on GPIO{STRIP8_PIN_NUM}, SOS on GPIO{BUILTIN_LED_PIN_NUM}");

    // todo0 Name mismatch
    let led_strip8 = LedStrip8::new(p.GPIO10, rmt80.channel0, spawner)?;
    #[cfg(target_arch = "riscv32")]
    let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel1, spawner)?;
    #[cfg(target_arch = "xtensa")]
    let sos_strip = SosStrip::new(p.GPIO48, rmt80.channel1, spawner)?;

    let frame = Frame1d([
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
        colors::BLUE,
        colors::GRAY,
    ]);
    led_strip8.write_frame(frame);
    sos_strip.animate(&SOS);

    core::future::pending().await
}
