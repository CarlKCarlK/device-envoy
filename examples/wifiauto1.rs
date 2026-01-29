//! Minimal WiFiAuto example based on the struct docs.

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use core::convert::Infallible;
use core::future;
use device_kit::{
    Result,
    button::PressedTo,
    flash_array::{FlashArray, FlashArrayStatic},
    wifi_auto::{WifiAuto, WifiAutoEvent},
};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    static FLASH_STATIC: FlashArrayStatic = FlashArray::<1>::new_static();
    let [wifi_flash] = FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    let wifi_auto = WifiAuto::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data
        p.PIO0,    // WiFi PIO
        p.DMA_CH0, // WiFi DMA
        wifi_flash,
        p.PIN_13, // Button for reconfiguration
        PressedTo::Ground,
        "PicoAccess", // Captive-portal SSID
        [],           // Any extra fields
        spawner,
    )?;

    let (_stack, _button) = wifi_auto
        .connect_with(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    defmt::info!("Captive portal ready");
                }
                WifiAutoEvent::Connecting { .. } => {
                    defmt::info!("Connecting to WiFi");
                }
                WifiAutoEvent::Connected => {
                    defmt::info!("WiFi connected");
                }
                WifiAutoEvent::ConnectionFailed => {
                    defmt::info!("WiFi connection failed");
                }
            }
            Ok(())
        })
        .await?;

    // The stack is ready for network operations (for example, NTP or HTTP).

    future::pending().await // run forever
}
