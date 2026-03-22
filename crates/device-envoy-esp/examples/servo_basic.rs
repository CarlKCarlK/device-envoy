//! Basic single-servo control example.
//!
//! Wiring:
//! - Servo signal -> GPIO10

#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;

use device_envoy_esp::{init_and_start, servo, servo::Servo as _, Result};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
servo! {
    BasicServo {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
    }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
servo! {
    BasicServo {
        pin: GPIO4,
        timer: Timer0,
        channel: Channel0,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);

    #[cfg(esp_gdma_family)]
    let basic_servo = BasicServo::new(&ledc, p.GPIO10)?;
    #[cfg(esp_pdma_family)]
    let basic_servo = BasicServo::new(&ledc, p.GPIO4)?;

    basic_servo.set_degrees(45); // Move to 45 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position

    basic_servo.set_degrees(90); // Move to 90 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position

    basic_servo.relax(); // Let the servo relax. It will re-enable on next set_degrees()

    core::future::pending().await
}
