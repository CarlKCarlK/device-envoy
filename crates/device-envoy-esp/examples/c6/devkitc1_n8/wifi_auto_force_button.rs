//! Demonstrates forcing WifiAuto captive-portal flow using a physical button.
//!
//! Wiring:
//! - GPIO6 <-> button <-> GND
//! - Use internal pull-up (`PressedTo::Ground`).

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    Result,
    button::{ButtonEsp, PressedTo},
    flash_block::FlashBlockEsp,
    init_and_start,
    wifi_auto::{WifiAuto as _, WifiAutoEsp, WifiAutoEvent},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        "DeviceEnvoySetup",
        [],
        spawner,
    )?;

    let _stack = wifi_auto
        .connect(&mut button, |wifi_auto_event| async move {
            match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => info!("Captive portal ready"),
                WifiAutoEvent::Connecting { .. } => info!("Connecting"),
                WifiAutoEvent::ConnectionFailed => info!("Connection failed"),
            }
            Ok(())
        })
        .await?;

    core::future::pending().await
}
