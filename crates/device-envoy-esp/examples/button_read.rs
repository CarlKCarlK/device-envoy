//! Button-read example for an external button on GPIO6.
//!
//! Wiring (recommended):
//! - GPIO6 <-> button <-> GND
//! - Uses internal pull-up, so press reads as active-low.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{Button as _, ButtonEsp, PressedTo, BUTTON_POLL_INTERVAL},
    init_and_start,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(_spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let mut was_pressed = button.is_pressed();
    info!(
        "button_read ready: GPIO6, pressed={} (PressedTo::Ground)",
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
