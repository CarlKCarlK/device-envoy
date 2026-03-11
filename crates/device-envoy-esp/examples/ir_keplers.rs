//! SunFounder Kepler IR remote demo with two receivers.
//!
//! Wiring:
//! - IR receiver #0 data output -> GPIO7
//! - IR receiver #1 data output -> GPIO4
//! - Both receiver VCC pins -> 3.3V
//! - Both receiver GND pins -> GND

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{init_and_start, ir::IrKepler as _, ir_keplers};

esp_bootloader_esp_idf::esp_app_desc!();

ir_keplers! {
    IrKeplers0 {
        IrKepler7: { pin: GPIO7 },
        IrKepler4: { pin: GPIO4 }
    }
}

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

    // On ESP32-S3, RMT channels 0–3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0–3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let channel_creator0 = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator0 = rmt80.channel2;

    #[cfg(target_arch = "xtensa")]
    let channel_creator1 = rmt80.channel5;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator1 = rmt80.channel3;

    let (ir_kepler7, ir_kepler4) = IrKeplers0::new(
        p.GPIO7,
        channel_creator0,
        p.GPIO4,
        channel_creator1,
        spawner,
    )?;

    info!("Kepler remotes initialized on GPIO7 and GPIO4");

    loop {
        match select(ir_kepler7.wait_for_press(), ir_kepler4.wait_for_press()).await {
            Either::First(button0) => info!("Kepler7 button: {:?}", button0),
            Either::Second(button1) => info!("Kepler4 button: {:?}", button1),
        }
    }
}
