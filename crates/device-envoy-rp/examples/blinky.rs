#![allow(missing_docs)]
//! Minimal async blink example for Raspberry Pi Pico 2.
//! Emits SOS in Morse code using the Led device abstraction.
//!
//! ## LED Wiring
//!
//! **Option 1: External LED on PIN_1 (current setup)**
//! - LED anode (long leg) → 220Ω resistor → PIN_1
//! - LED cathode (short leg) → GND
//!
//! **Option 2: Onboard LED (non-W Pico boards)**
//! - For non-WiFi Pico boards, change `p.PIN_1` to `p.PIN_25` to use the onboard LED
//! - No external wiring needed
#![no_std]
#![no_main]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy::{
    Result,
    led::{Led, LedStatic, OnLevel},
};
use embassy_executor::Spawner;
use embassy_rp::gpio::Level;
use embassy_time::Duration;
use panic_probe as _;

const DOT_MS: u64 = 200;
const DASH_MS: u64 = DOT_MS * 3;
const SYMBOL_GAP_MS: u64 = DOT_MS;
const LETTER_GAP_MS: u64 = DOT_MS * 3;
const WORD_GAP_MS: u64 = DOT_MS * 7;

const DOT_DURATION: Duration = Duration::from_millis(DOT_MS);
const DASH_DURATION: Duration = Duration::from_millis(DASH_MS);
const SYMBOL_GAP_DURATION: Duration = Duration::from_millis(SYMBOL_GAP_MS);
const LETTER_GAP_DURATION: Duration = Duration::from_millis(LETTER_GAP_MS);
const WORD_GAP_DURATION: Duration = Duration::from_millis(WORD_GAP_MS);

const SOS_PATTERN: [(Level, Duration); 18] = [
    // S: dot dot dot
    (Level::High, DOT_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DOT_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DOT_DURATION),
    (Level::Low, LETTER_GAP_DURATION),
    // O: dash dash dash
    (Level::High, DASH_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DASH_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DASH_DURATION),
    (Level::Low, LETTER_GAP_DURATION),
    // S: dot dot dot
    (Level::High, DOT_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DOT_DURATION),
    (Level::Low, SYMBOL_GAP_DURATION),
    (Level::High, DOT_DURATION),
    (Level::Low, WORD_GAP_DURATION),
];

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    static LED_STATIC: LedStatic = Led::new_static();
    let led = Led::new(&LED_STATIC, p.PIN_1, OnLevel::High, spawner)?;

    info!("Emitting SOS in Morse code");
    led.animate(&SOS_PATTERN);

    // Animation loops continuously in background task
    core::future::pending().await
}
