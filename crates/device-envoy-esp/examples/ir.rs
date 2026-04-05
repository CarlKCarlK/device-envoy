//! Basic NEC IR receiver demo.
//!
//! Wiring:
//! - IR receiver data output -> GPIO7
//! - IR receiver VCC -> 3.3V
//! - IR receiver GND -> GND

#![no_std]
#![no_main]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    init_and_start, ir,
    ir::{Ir as _, IrEvent},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
ir! {
    Ir7 { pin: GPIO7 }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
ir! {
    Ir7 { pin: GPIO4 }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    info!("ir example started: listening on GPIO7");

    // ESP32-S3 requires RX channel 4+.
    #[cfg(feature = "esp32s3")]
    let channel_creator = rmt80.channel4;
    #[cfg(not(feature = "esp32s3"))]
    let channel_creator = rmt80.channel2;
    #[cfg(esp_gdma_family)]
    let ir7 = Ir7::new(p.GPIO7, channel_creator, spawner)?;
    #[cfg(esp_pdma_family)]
    let ir7 = Ir7::new(p.GPIO4, channel_creator, spawner)?;

    loop {
        let IrEvent::Press { addr, cmd } = ir7.wait_for_press().await;
        info!("IR press: addr=0x{:04X}, cmd=0x{:02X}", addr, cmd);
    }
}
