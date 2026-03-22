#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start, servo,
    servo::Servo,
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
servo! {
    Servo10 {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
    }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
servo! {
    Servo10 {
        pin: GPIO4,
        timer: Timer0,
        channel: Channel0,
    }
}

async fn move_and_relax(servo: &impl Servo) {
    servo.set_degrees(45); // Move to 45 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position
    servo.set_degrees(90); // Move to 90 degrees and hold.
    Timer::after(Duration::from_secs(1)).await; // Give servo reasonable time to reach position
    servo.relax(); // Let the servo relax. It will re-enable on next set_degrees()
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

    #[cfg(esp_gdma_family)]
    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    #[cfg(esp_pdma_family)]
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
    #[cfg(esp_gdma_family)]
    let servo10 = Servo10::new(&ledc, p.GPIO10)?;
    #[cfg(esp_pdma_family)]
    let servo10 = Servo10::new(&ledc, p.GPIO4)?;

    loop {
        move_and_relax(&servo10).await;
        info!("Press the button to run the servo sequence again.");
        button.wait_for_press().await;
    }
}
