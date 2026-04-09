//! Wiring:
//! - Probe LED or scope on GPIO4 and GPIO6
//!
#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use log::info;

use device_envoy_esp::{Result, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("C3 LED probe starting: GPIO4 (D4?) and GPIO6 (D6?)");
    info!("Observe which LED turns on for each phase to detect pin mapping and polarity");

    let mut led_d4 = Output::new(p.GPIO4, Level::Low, OutputConfig::default());
    let mut led_d6 = Output::new(p.GPIO6, Level::Low, OutputConfig::default());

    loop {
        info!("phase 1: GPIO4=HIGH, GPIO6=LOW");
        led_d4.set_high();
        led_d6.set_low();
        Timer::after(Duration::from_secs(2)).await;

        info!("phase 2: GPIO4=LOW, GPIO6=HIGH");
        led_d4.set_low();
        led_d6.set_high();
        Timer::after(Duration::from_secs(2)).await;

        info!("phase 3: GPIO4=HIGH, GPIO6=HIGH");
        led_d4.set_high();
        led_d6.set_high();
        Timer::after(Duration::from_secs(2)).await;

        info!("phase 4: GPIO4=LOW, GPIO6=LOW");
        led_d4.set_low();
        led_d6.set_low();
        Timer::after(Duration::from_secs(2)).await;
    }
}
