//! Dual servo control example.
//!
//! Moves two servos in opposite directions for 2 seconds.
//!
//! Wiring:
//! - Servo A signal -> GPIO10
//! - Servo B signal -> GPIO18

#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{init_and_start, servo, servo::Servo as _, Result};

esp_bootloader_esp_idf::esp_app_desc!();

servo! {
    ServoA {
        timer: Timer0,
        channel: Channel0,
    }
}

servo! {
    ServoB {
        timer: Timer1,
        channel: Channel1,
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
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("Starting dual servo example");

    let mut servo_a = ServoA::new(&ledc, p.GPIO10)?;
    let mut servo_b = ServoB::new(&ledc, p.GPIO18)?;

    info!("Moving servos in opposite directions for 2 seconds");

    let start = Instant::now();
    let duration = Duration::from_secs(2);

    loop {
        if start.elapsed() > duration {
            break;
        }

        servo_a.set_degrees(0);
        servo_b.set_degrees(180);
        Timer::after_millis(500).await;

        servo_a.set_degrees(180);
        servo_b.set_degrees(0);
        Timer::after_millis(500).await;
    }

    info!("Centering servos");
    servo_a.set_degrees(90);
    servo_b.set_degrees(90);
    Timer::after_millis(500).await;

    info!("Relaxing servos");
    servo_a.relax();
    servo_b.relax();

    core::future::pending().await
}
