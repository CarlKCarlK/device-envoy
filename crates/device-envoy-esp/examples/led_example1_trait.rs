#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_core::led::Led as _;
use device_envoy_esp::{
    init_and_start,
    led::{LedEsp, LedEspStatic, LedLevel, OnLevel},
};
use embassy_time::{Duration, Timer};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("led_example1_trait: blinking external LED on GPIO2");

    static LED_ESP_STATIC: LedEspStatic = LedEsp::new_static();
    let led_esp = LedEsp::new(&LED_ESP_STATIC, p.GPIO2, OnLevel::High, spawner)?;

    // Turn the LED on
    led_esp.set_level(LedLevel::On);
    Timer::after(Duration::from_secs(1)).await;

    // Turn the LED off
    led_esp.set_level(LedLevel::Off);
    Timer::after(Duration::from_millis(500)).await;

    // Play a blinking animation (looping: 200ms on, 200ms off)
    led_esp.animate([
        (LedLevel::On, Duration::from_millis(200)),
        (LedLevel::Off, Duration::from_millis(200)),
    ]);

    core::future::pending().await
}
