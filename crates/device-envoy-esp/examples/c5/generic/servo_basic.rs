//! Basic single-servo control example.
//!
//! Wiring:
//! - Servo signal -> GPIO10
//! - Servo power -> 5V (do not use 3.3V for typical hobby servos)
//! - Servo ground -> GND (shared with ESP32 GND)
//! - If using a separate 5V supply, connect supply GND to ESP32 GND (common ground required)
//! - Do not power a servo directly from a GPIO pin
//! - Common red/brown/yellow wiring:
//!   red -> 5V, brown -> GND, yellow -> signal (GPIO10)

#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, servo, servo::Servo as _, Result};

esp_bootloader_esp_idf::esp_app_desc!();

servo! {
    BasicServo {
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

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);

    let basic_servo = BasicServo::new(&ledc, p.GPIO10)?;

    basic_servo.set_degrees(45); // Move to 45 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position

    basic_servo.set_degrees(90); // Move to 90 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position

    basic_servo.relax(); // Let the servo relax. It will re-enable on next set_degrees()

    pending().await
}
