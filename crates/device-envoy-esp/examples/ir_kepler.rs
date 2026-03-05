//! SunFounder Kepler IR remote demo.
//!
//! Wiring:
//! - IR receiver data output -> GPIO7
//! - IR receiver VCC -> 3.3V
//! - IR receiver GND -> GND

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start,
    ir::{IrKepler as _, IrKeplerEsp, IrKeplerStatic},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80, rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    static IR_KEPLER_STATIC: IrKeplerStatic = IrKeplerEsp::new_static();
    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let ir_rmt_channel = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let ir_rmt_channel = rmt80.channel2;
    let ir_kepler = IrKeplerEsp::new(&IR_KEPLER_STATIC, p.GPIO7, ir_rmt_channel, spawner)?;

    loop {
        let button = ir_kepler.wait_for_press().await;
        info!("Kepler button: {:?}", button);
    }
}
