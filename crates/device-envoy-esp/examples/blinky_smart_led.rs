//! SOS blinky for a smart LED.
//!
//! Setup:
//! - Connect one smart-LED data input to the LED pin listed below.
//! - This example always drives one pixel (`len: 1`), even if your strip has more.
//! - LuatOS ESP32-C3 boards typically use external LEDs, so wire one to the C3 pin below.
//!
//! Default pin mapping:
//! - ESP32-C3: GPIO2 (matches ESP Rust board smart-LED pin)
//! - ESP32-S3: GPIO38 (common built-in smart-LED pin on S3 dev boards)
//! - RMT + GDMA-family chips (except ESP32-C3/S3): GPIO8
//! - RMT + PDMA-family chips (ESP32/S2): GPIO0
//! - Non-RMT chips (currently ESP32-C2): GPIO0 via SPI

#![no_std]
#![no_main]

#[cfg(not(esp_has_rmt))]
#[allow(unused_imports)]
use device_envoy_esp::led_strip::Engine;
use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{colors, Frame1d, LedStrip as _},
    Result,
};
use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(all(
    not(rust_analyzer),
    esp_has_rmt,
    not(any(esp_gdma_family, esp_pdma_family))
))]
compile_error!("Unsupported RMT family. Add a blinky_smart_led pin mapping for this chip family.");

#[cfg(all(not(rust_analyzer), not(esp_has_rmt), not(feature = "esp32c2")))]
compile_error!(
    "Non-RMT blinky_smart_led currently supports only esp32c2. Add a transport/pin mapping for this chip."
);

#[cfg(feature = "esp32c3")]
const LED_PIN_NUM: u8 = 2;
#[cfg(feature = "esp32s3")]
const LED_PIN_NUM: u8 = 38;
#[cfg(all(
    not(any(feature = "esp32c3", feature = "esp32s3")),
    esp_has_rmt,
    esp_gdma_family
))] // C6, H2
const LED_PIN_NUM: u8 = 8;
#[cfg(any(all(esp_has_rmt, esp_pdma_family), not(esp_has_rmt)))] // original ESP32 & S2, and non-RMT (currently ESP32-C2)
const LED_PIN_NUM: u8 = 0;

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
led_strip! {
    SosStrip {
        pin: GPIO2,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(feature = "esp32s3")]
led_strip! {
    SosStrip {
        pin: GPIO38,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(all(
    not(any(feature = "esp32c3", feature = "esp32s3")),
    esp_has_rmt,
    esp_gdma_family
))]
led_strip! {
    SosStrip {
        pin: GPIO8,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(all(esp_has_rmt, esp_pdma_family))]
led_strip! {
    SosStrip {
        pin: GPIO0,
        len: 1,
        max_frames: 20,
    }
}

#[cfg(not(esp_has_rmt))]
led_strip! {
    SosStrip {
        pin: GPIO0,
        len: 1,
        engine: Engine::Spi,
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
    #[cfg(esp_has_rmt)]
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    #[cfg(not(esp_has_rmt))]
    init_and_start!(p);

    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("SOS smart-LED blinky starting on GPIO{LED_PIN_NUM}");

    #[cfg(feature = "esp32c3")]
    let sos_strip = SosStrip::new(p.GPIO2, rmt80.channel0, spawner)?;
    #[cfg(feature = "esp32s3")]
    let sos_strip = SosStrip::new(p.GPIO38, rmt80.channel0, spawner)?;
    #[cfg(all(
        not(any(feature = "esp32c3", feature = "esp32s3")),
        esp_has_rmt,
        esp_gdma_family
    ))]
    let sos_strip = SosStrip::new(p.GPIO8, rmt80.channel0, spawner)?;
    #[cfg(all(esp_has_rmt, esp_pdma_family))]
    let sos_strip = SosStrip::new(p.GPIO0, rmt80.channel0, spawner)?;
    #[cfg(not(esp_has_rmt))]
    let sos_strip = SosStrip::new(p.GPIO0, p.SPI2, spawner)?;

    sos_strip.animate(&SOS);

    core::future::pending().await
}
