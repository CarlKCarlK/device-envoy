//! Wiring:
//! - No external wiring required; this example only uses the onboard Wi-Fi radio.
//!
//! Wi-Fi bring-up smoke test using `esp-radio` on ESP32-S2.
//! If this chip/board profile does not support Wi-Fi, use `wifi_auto_*` examples on a supported profile.
//!
//! This example initializes the Wi-Fi driver and performs one AP scan.
//!
//! Diagnostic note: this is a low-level `esp-radio` scan example.
//! For application-level flows, prefer the `wifi_auto_*` examples.

#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{Result, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

const WIFI_HEAP_BYTES: usize = 72 * 1024;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    esp_alloc::heap_allocator!(size: WIFI_HEAP_BYTES);

    let (mut wifi_controller, _interfaces) =
        esp_radio::wifi::new(p.WIFI, Default::default()).expect("esp_radio::wifi::new failed");

    wifi_controller
        .set_config(&esp_radio::wifi::Config::Station(
            esp_radio::wifi::sta::StationConfig::default(),
        ))
        .expect("set Wi-Fi client mode failed");

    let scan_config = esp_radio::wifi::scan::ScanConfig::default();
    let scan_results = wifi_controller
        .scan_async(&scan_config)
        .await
        .expect("Wi-Fi scan failed");

    info!("wifi_scan: found {} APs", scan_results.len());
    for access_point_info in scan_results.iter().take(10) {
        info!(
            "ssid='{:?}' rssi={} channel={} auth={:?}",
            access_point_info.ssid,
            access_point_info.signal_strength,
            access_point_info.channel,
            access_point_info.auth_method
        );
    }

    pending().await
}
