//! Basic NEC IR receiver demo.
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
    init_and_start, ir,
    ir::{Ir as _, IrEvent},
};

esp_bootloader_esp_idf::esp_app_desc!();

ir! {
    Ir7: { pin: GPIO7 }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("ir example started: listening on GPIO7");

    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let channel_creator = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator = rmt80.channel2;
    let ir7 = Ir7::new(p.GPIO7, channel_creator, spawner)?;

    loop {
        let IrEvent::Press { addr, cmd } = ir7.wait_for_press().await;
        info!("IR press: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
    }
}
