//! Wiring:
//! - External LED anode -> resistor -> GPIO3
//! - External LED cathode -> GND
//!
#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_core::led::Led as _;
use device_envoy_esp::{
    Result, init_and_start, led,
    led::{LedLevel, OnLevel},
};
use embassy_time::{Duration, Timer};

led! {
    pub LedExample {
        pin: GPIO3,
        max_steps: 64
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("led_example1: blinking external LED on GPIO3");

    let led_example = LedExample::new(p.GPIO3, OnLevel::High, spawner)?;

    // Turn the LED on
    led_example.set_level(LedLevel::On);
    Timer::after(Duration::from_secs(1)).await;

    // Turn the LED off
    led_example.set_level(LedLevel::Off);
    Timer::after(Duration::from_millis(500)).await;

    // Play a blinking animation (looping: 200ms on, 200ms off)
    led_example.animate([
        (LedLevel::On, Duration::from_millis(200)),
        (LedLevel::Off, Duration::from_millis(200)),
    ]);

    pending().await
}
