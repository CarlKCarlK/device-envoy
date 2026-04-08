// @generated examples/templates/audio20k_one.rs.j2 by cargo xtask generate-board-examples.
#![allow(missing_docs)]
//! audio20k_one: one short 22.05 kHz clip, then stop.
//!
//! Purpose:
//! - Tail-repeat verification for `AtEnd::Stop`
//!
//! Wiring (audio-capable boards):
//! - Audio data pin (`DIN`) -> GPIO0
//! - Audio bit clock pin (`BCLK`) -> GPIO0
//! - Audio word select pin (`LRC` / `LRCLK`) -> GPIO0

#![no_std]
#![no_main]

use core::{convert::Infallible, future::pending};

use embassy_executor::Spawner;
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{Result, init_and_start};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let _ = spawner;
    info!("audio20k_one: audio not supported on this board profile");

    pending().await
}
