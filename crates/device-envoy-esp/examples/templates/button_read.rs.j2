//! Button-read example for an external button.
//!
//! Wiring (recommended):
//! - `esp_gdma_family` (ESP32-C3/C6/S3): GPIO6 <-> button <-> GND
//! - `esp_pdma_family` (ESP32/S2): GPIO0 <-> button <-> GND
//! - Uses internal pull-up, so press reads as active-low.

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{Button as _, ButtonEsp, PressedTo, BUTTON_POLL_INTERVAL},
    init_and_start, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    #[cfg(esp_gdma_family)]
    let button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    #[cfg(esp_pdma_family)]
    let button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
    let mut was_pressed = button.is_pressed();
    info!(
        "button_read ready: pressed={} (PressedTo::Ground)",
        was_pressed
    );

    loop {
        let is_pressed = button.is_pressed();
        if is_pressed != was_pressed {
            info!("button pressed={}", is_pressed);
            was_pressed = is_pressed;
        }
        Timer::after(BUTTON_POLL_INTERVAL).await;
    }
}
