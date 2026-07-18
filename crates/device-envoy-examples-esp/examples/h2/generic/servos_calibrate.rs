
//! Dual-servo calibration helper.
//!
//! Behavior:
//! - On boot, sweeps both servos 0 -> 180 -> 0.
//! - Short press: add 45 degrees.
//! - At 180 degrees, the next short press wraps to 0.
//! - Long press: reset both servos to 0.
//!
//! Wiring:
//! - Button -> GPIO0 to GND (`PressedTo::Ground`)
//! - Servo A signal -> GPIO10
//! - Servo B signal -> GPIO1
//! - Servo power -> 5V (do not use 3.3V for typical hobby servos)
//! - Servo ground -> GND (shared with ESP32 GND)
//! - If using a separate 5V supply, connect supply GND to ESP32 GND (common ground required)
//! - Do not power a servo directly from a GPIO pin
//! - Common red/brown/yellow wiring (each servo):
//!   red -> 5V, brown -> GND, yellow -> signal (GPIO10 / GPIO1)

#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_core::button::{Button, PressDuration};
use device_envoy_esp::{
    button::{ButtonEsp, PressedTo},
    init_and_start,
    servo,
    servo::{Servo as _, ServoEsp},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

servo! {
    ServoA {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
    }
}

servo! {
    ServoB {
        pin: GPIO1,
        timer: Timer1,
        channel: Channel1,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("servos_calibrate: starting");
    info!(
        "servos_calibrate: button=GPIO{}, servo_a=GPIO{}, servo_b=GPIO{}",
        0,
        10,
        1
    );

    let servo_a = ServoA::new(&ledc, p.GPIO10)?;
    let servo_b = ServoB::new(&ledc, p.GPIO1)?;
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);

    info!("servos_calibrate: startup sweep 0 -> 180 -> 0");
    set_both(&servo_a, &servo_b, 0);
    Timer::after(Duration::from_millis(500)).await;
    set_both(&servo_a, &servo_b, 180);
    Timer::after(Duration::from_millis(900)).await;
    set_both(&servo_a, &servo_b, 0);
    Timer::after(Duration::from_millis(500)).await;

    let mut degrees: u16 = 0;
    info!("servos_calibrate: ready; short=+45, long=reset to 0");

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                let next_degrees = if degrees == 180 { 0 } else { degrees + 45 };
                info!(
                    "servos_calibrate: short press; degrees {} -> {}",
                    degrees, next_degrees
                );
                degrees = next_degrees;
                set_both(&servo_a, &servo_b, degrees);
            }
            PressDuration::Long => {
                info!(
                    "servos_calibrate: long press; reset degrees {} -> 0",
                    degrees
                );
                degrees = 0;
                set_both(&servo_a, &servo_b, degrees);
            }
        }
    }
}

fn set_both(servo_a: &ServoEsp, servo_b: &ServoEsp, degrees: u16) {
    info!(
        "servos_calibrate: set both servos to {} degrees (GPIO10, GPIO1)",
        degrees
    );
    servo_a.set_degrees(degrees);
    servo_b.set_degrees(degrees);
}