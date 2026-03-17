//! Demonstrates forcing WifiAuto captive-portal flow using a physical button.
//!
//! Wiring:
//! - GPIO6 <-> button <-> GND
//! - Use internal pull-up (`PressedTo::Ground`).

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{ButtonEsp, PressedTo},
    flash_block::FlashBlockEsp,
    init_and_start,
    wifi_auto::{WifiAuto as _, WifiAutoEsp, WifiAutoEvent},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<core::convert::Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        "DeviceEnvoySetup",
        [],
        spawner,
    )?;

    let _stack = wifi_auto
        .connect(&mut button6, |wifi_auto_event| async move {
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
