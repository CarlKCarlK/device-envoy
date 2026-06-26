#![allow(missing_docs)]
//! Minimal WiFiAuto example with a no-op event handler.

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use core::{convert::Infallible, future::pending};
use device_envoy_rp::{
    Result,
    button::{ButtonRp, PressedTo},
    flash_block::FlashBlockRp,
    wifi_auto::WifiAutoRp,
};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let [wifi_flash] = FlashBlockRp::new_array::<1>(p.FLASH)?;

    let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 data
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 clock
        p.PIO0,    // WiFi PIO
        p.DMA_CH0, // WiFi DMA
        wifi_flash,
        "DeviceEnvoySetup", // Captive-portal SSID
        [],                 // Any extra fields
        spawner,
    )?;

    let _stack = wifi_auto
        .connect(&mut button, async |_event| {
            Ok::<(), device_envoy_rp::Error>(())
        })
        .await?;

    pending().await // run forever
}
