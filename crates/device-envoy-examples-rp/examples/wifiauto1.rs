#![allow(missing_docs)]
//! Minimal WiFiAuto example based on the struct docs.

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

// TODO consider migrating this legacy DNS example to device_envoy_core::dns::{Dns, DnsRuntime}.

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use core::convert::Infallible;
use device_envoy_rp::{
    Result,
    button::{ButtonRp, PressedTo},
    flash_block::FlashBlockRp,
    wifi_auto::{WifiAutoEvent, WifiAutoRp},
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

    let [wifi_credentials_flash_block] = FlashBlockRp::new_array::<1>(p.FLASH)?;

    let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 data
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 clock
        p.PIO0,    // WiFi PIO
        p.DMA_CH0, // WiFi DMA
        wifi_credentials_flash_block,
        "DeviceEnvoySetup", // Captive-portal SSID
        [],                 // Any extra fields
        spawner,
    )?;

    let stack = wifi_auto
        .connect(
            &mut button,
            async |event| -> Result<(), device_envoy_rp::Error> {
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
            },
        )
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
