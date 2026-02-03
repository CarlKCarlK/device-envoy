#![allow(missing_docs)]
//! Minimal WiFiAuto example based on the struct docs.

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use core::convert::Infallible;
use device_kit::{
    Result,
    button::PressedTo,
    flash_array::FlashArray,
    wifi_auto::{WifiAuto, WifiAutoEvent},
};
use embassy_net::dns::DnsQueryType;
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    let [wifi_flash] = FlashArray::<1>::new(p.FLASH)?;

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

    let (stack, _button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    defmt::info!("Captive portal ready");
                }
                WifiAutoEvent::Connecting { .. } => {
                    defmt::info!("Connecting to WiFi");
                }
                WifiAutoEvent::ConnectionFailed => {
                    defmt::info!("WiFi connection failed");
                }
            }
            Ok(())
        })
        .await?;

    // The stack is ready for network operations (for example, NTP or HTTP).
    defmt::info!("WiFi connected");

    loop {
        if let Ok(addresses) = stack.dns_query("google.com", DnsQueryType::A).await {
            defmt::info!("google.com: {:?}", addresses);
        } else {
            defmt::info!("google.com: lookup failed");
        }
        Timer::after(Duration::from_secs(15)).await;
    }
}
