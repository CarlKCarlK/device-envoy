//! Wiring:
//! - Servo signal -> GPIO10
//! - Servo power -> 5V (do not use 3.3V for typical hobby servos)
//! - Servo ground -> GND (shared with ESP32 GND)
//! - If using a separate 5V supply, connect supply GND to ESP32 GND (common ground required)
//! - Do not power a servo directly from a GPIO pin
//! - Common red/brown/yellow wiring:
//!   red -> 5V, brown -> GND, yellow -> signal (GPIO10)
//! - Button -> GPIO6 to GND (`PressedTo::Ground`)
//!
#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use log::info;

use device_envoy_core::servo::{Servo, ServoPlayer};
use device_envoy_esp::{
    Result,
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start,
    servo::{AtEnd, combine, linear, servo_player},
};

esp_bootloader_esp_idf::esp_app_desc!();

servo_player! {
    ServoSweep {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
        min_us: 500,
        max_us: 2500,
        max_degrees: 180,
        max_steps: 40,
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

    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let servo_sweep = ServoSweep::new(&ledc, p.GPIO10, spawner)?;

    loop {
        // Combine 40 animation steps into one array.
        const STEPS_UP_AND_HOLD: [(u16, Duration); 20] = combine::<19, 1, 20>(
            linear::<19>(0, 180, Duration::from_secs(2)), // 19 steps from 0 degrees to 180 degrees
            [(180, Duration::from_millis(400))],          // Hold at 180 degrees for 400 ms
        );
        const STEPS_DOWN_AND_HOLD: [(u16, Duration); 20] = combine::<19, 1, 20>(
            linear::<19>(180, 0, Duration::from_secs(2)), // 19 steps from 180 degrees to 0 degrees
            [(0, Duration::from_millis(400))],            // Hold at 0 degrees for 400 ms
        );
        const STEPS: [(u16, Duration); 40] =
            combine::<20, 20, 40>(STEPS_UP_AND_HOLD, STEPS_DOWN_AND_HOLD);

        servo_sweep.animate(STEPS, AtEnd::Loop); // Loop the sweep animation

        // Let it run in the background for 10 seconds, then relax.
        embassy_time::Timer::after(Duration::from_secs(10)).await;
        servo_sweep.relax();

        info!("Press the button to run the servo sweep again.");
        button.wait_for_press().await;
    }
}
