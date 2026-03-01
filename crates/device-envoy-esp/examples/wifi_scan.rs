//! Wi-Fi bring-up smoke test using `esp-radio` on ESP32-C6.
//!
//! This example initializes the Wi-Fi driver and performs one AP scan.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp32::init_and_start;

esp_bootloader_esp_idf::esp_app_desc!();

const WIFI_HEAP_BYTES: usize = 72 * 1024;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(_spawner: Spawner) -> device_envoy_esp32::Result<core::convert::Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    esp_alloc::heap_allocator!(size: WIFI_HEAP_BYTES);

    let esp_radio_controller = esp_radio::init().expect("esp_radio::init failed");
    let (mut wifi_controller, _interfaces) = esp_radio::wifi::new(
        &esp_radio_controller,
        p.WIFI,
        esp_radio::wifi::Config::default(),
    )
    .expect("esp_radio::wifi::new failed");

    wifi_controller
        .set_config(&esp_radio::wifi::ModeConfig::Client(
            esp_radio::wifi::ClientConfig::default(),
        ))
        .expect("set Wi-Fi client mode failed");

    wifi_controller
        .start_async()
        .await
        .expect("starting Wi-Fi failed");

    let scan_results = wifi_controller
        .scan_with_config_async(esp_radio::wifi::ScanConfig::default())
        .await
        .expect("Wi-Fi scan failed");

    info!("wifi_scan: found {} APs", scan_results.len());
    for access_point_info in scan_results.iter().take(10) {
        info!(
            "ssid='{}' rssi={} channel={} auth={:?}",
            access_point_info.ssid,
            access_point_info.signal_strength,
            access_point_info.channel,
            access_point_info.auth_method
        );
    }

    core::future::pending().await
}
