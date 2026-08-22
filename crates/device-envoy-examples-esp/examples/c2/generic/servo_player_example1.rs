//! Wiring:
//! - Servo signal -> GPIO10
//! - Servo power -> 5V (do not use 3.3V for typical hobby servos)
//! - Servo ground -> GND (shared with ESP32 GND)
//! - If using a separate 5V supply, connect supply GND to ESP32 GND (common ground required)
//! - Do not power a servo directly from a GPIO pin
//! - Common red/brown/yellow wiring:
//!   red -> 5V, brown -> GND, yellow -> signal (GPIO10)
//! - Button -> GPIO18 to GND (`PressedTo::Ground`)
//!
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_core::servo::{Servo, ServoPlayer};
use device_envoy_esp::{
    Result,
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start,
    servo::{AtEnd, servo_player},
};

esp_bootloader_esp_idf::esp_app_desc!();

servo_player! {
    ServoPlayer10 {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let mut button = ButtonEsp::new(p.GPIO18, PressedTo::Ground);
    let servo_player10 = ServoPlayer10::new(&ledc, p.GPIO10, spawner)?;

    loop {
        // Move to 90 degrees, wait 1 second, then relax.
        servo_player10.set_degrees(90);
        Timer::after(Duration::from_secs(1)).await;
        servo_player10.relax();

        // Animate: hold at 180 degrees for 1 second, then 0 degrees for 1 second, then relax.
        const STEPS: [(u16, Duration); 2] =
            [(180, Duration::from_secs(1)), (0, Duration::from_secs(1))];
        // AtEnd::Relax quiets the servo; AtEnd::Hold keeps driving pulses to hold
        // position; AtEnd::Loop repeats.
        servo_player10.animate(STEPS, AtEnd::Relax);

        info!("Press the button to run the servo player sequence again.");
        button.wait_for_press().await;
    }
}
