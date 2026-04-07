#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_core::servo::ServoPlayer;
use device_envoy_esp::{
    Result,
    button::{Button as _, ButtonEsp, PressedTo},
    init_and_start,
    servo::{AtEnd, servo_player},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
servo_player! {
    ServoPlayer10 {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
    }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
servo_player! {
    ServoPlayer10 {
        pin: GPIO4,
        timer: Timer0,
        channel: Channel0,
    }
}

async fn basic_servo_control<const MAX_STEPS: usize>(servo_player: &impl ServoPlayer<MAX_STEPS>) {
    // Move to 90 degrees, wait 1 second, then relax.
    servo_player.set_degrees(90);
    Timer::after(Duration::from_secs(1)).await;
    servo_player.relax();

    // Animate: hold at 180 degrees for 1 second, then 0 degrees for 1 second, then relax.
    const STEPS: [(u16, Duration); 2] =
        [(180, Duration::from_secs(1)), (0, Duration::from_secs(1))];
    // AtEnd::Relax quiets the servo; AtEnd::Hold keeps driving pulses to hold
    // position; AtEnd::Loop repeats.
    servo_player.animate(STEPS, AtEnd::Relax);
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    #[cfg(esp_gdma_family)]
    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    #[cfg(esp_pdma_family)]
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
    #[cfg(esp_gdma_family)]
    let servo_player10 = ServoPlayer10::new(&ledc, p.GPIO10, spawner)?;
    #[cfg(esp_pdma_family)]
    let servo_player10 = ServoPlayer10::new(&ledc, p.GPIO4, spawner)?;

    loop {
        basic_servo_control(&servo_player10).await;
        info!("Press the button to run the servo player sequence again.");
        button.wait_for_press().await;
    }
}
