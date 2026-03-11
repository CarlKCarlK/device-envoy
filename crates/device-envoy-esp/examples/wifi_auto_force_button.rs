//! Demonstrates forcing WifiAuto startup mode to captive portal using a button.
//!
//! Wiring:
//! - GPIO6 <-> button <-> GND
//! - Use internal pull-up (`PressedTo::Ground`).

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{flash_block::FlashBlockEsp, init_and_start, wifi_auto::WifiAutoEsp};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    let wifi_auto = WifiAutoEsp::new(p.WIFI, wifi_auto_flash_block, "EnvoySetup", [], spawner)?;

    let before_mode = wifi_auto.start_mode()?;
    let changed = wifi_auto.force_captive_portal_if_pressed_state(true)?;
    let after_mode = wifi_auto.start_mode()?;

    info!("wifi_auto_force_button");
    info!("  before_mode={:?}", before_mode);
    info!("  changed={}", changed);
    info!("  after_mode={:?}", after_mode);

    core::future::pending().await
}
