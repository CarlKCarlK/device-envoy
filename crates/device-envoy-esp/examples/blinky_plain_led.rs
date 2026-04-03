//! SOS blinky for a plain LED (non-smart LED).
//!
//! Setup:
//! - Wire an external LED as active-high:
//!   - LED anode (long leg) -> resistor -> LED pin
//!   - LED cathode (short leg) -> GND
//! - If your board has a built-in plain LED that is active-low, change `LED_ON_LEVEL`
//!   to `OnLevel::Low`.
//! - LuatOS ESP32-C3 boards typically use external LEDs, so wire one to the pin below.
//!
//! Default pin mapping:
//! - ESP32-C3: GPIO7 (matches ESP Rust board plain-LED pin)
//! - GDMA-family chips (C6/H2/S3): GPIO8
//! - PDMA-family chips (ESP32/S2): GPIO0
//!
//! These are reference-board defaults so this example can build for all supported chips.
//! If your board uses a different LED pin, change the pin mapping below.

#![no_std]
#![no_main]

use device_envoy_esp::{
    init_and_start, led,
    led::{Led as _, LedLevel, OnLevel},
    Result,
};
use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

// Board setup:
// - `LED_ON_LEVEL` is the user-facing polarity knob.
// - `LED_PIN_NUM` has per-family defaults for broad compile coverage; customize for your board.
const LED_ON_LEVEL: OnLevel = OnLevel::High;

#[cfg(feature = "esp32c3")]
const LED_PIN_NUM: u8 = 7;
#[cfg(all(not(feature = "esp32c3"), esp_gdma_family))] // C2, C6, H2, S3
const LED_PIN_NUM: u8 = 8;
#[cfg(all(not(feature = "esp32c3"), esp_pdma_family))] // original ESP32 & S2
const LED_PIN_NUM: u8 = 0;

#[cfg(all(
    not(rust_analyzer),
    target_os = "none",
    not(feature = "esp32c3"),
    not(any(esp_gdma_family, esp_pdma_family))
))]
compile_error!(
    "Unsupported chip family for blinky_plain_led pin mapping. Add a pin mapping for this family."
);

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

#[cfg(feature = "esp32c3")]
led! {
    SosLed {
        pin: GPIO7,
        max_steps: 20,
    }
}

#[cfg(all(not(feature = "esp32c3"), esp_gdma_family))]
led! {
    SosLed {
        pin: GPIO8,
        max_steps: 20,
    }
}

#[cfg(all(not(feature = "esp32c3"), esp_pdma_family))]
led! {
    SosLed {
        pin: GPIO0,
        max_steps: 20,
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
    init_and_start!(p);

    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("SOS plain LED blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(feature = "esp32c3")]
    let sos_led = SosLed::new(p.GPIO7, LED_ON_LEVEL, spawner)?;
    #[cfg(all(not(feature = "esp32c3"), esp_gdma_family))]
    let sos_led = SosLed::new(p.GPIO8, LED_ON_LEVEL, spawner)?;
    #[cfg(all(not(feature = "esp32c3"), esp_pdma_family))]
    let sos_led = SosLed::new(p.GPIO0, LED_ON_LEVEL, spawner)?;

    sos_led.animate(SOS);

    core::future::pending().await
}
