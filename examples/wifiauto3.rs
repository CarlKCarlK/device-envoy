//! WiFiAuto example with a custom web name field for DNS lookups.

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
    flash_array::{FlashArray, FlashArrayStatic},
    wifi_auto::{WifiAuto, WifiAutoEvent},
    wifi_auto::fields::{TextField, TextFieldStatic},
};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    static FLASH_STATIC: FlashArrayStatic = FlashArray::<2>::new_static();
    let [wifi_flash, web_name_flash] = FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    static WEB_NAME_STATIC: TextFieldStatic<32> = TextField::new_static();
    let web_name_field = TextField::new(
        &WEB_NAME_STATIC,
        web_name_flash,
        "web_name",
        "Web Name",
        "google.com",
    );

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
        "PicoAccess",      // Captive-portal SSID
        [web_name_field],  // Custom fields
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

    defmt::info!("WiFi connected");

    let web_name = web_name_field
        .text()?
        .unwrap_or_else(|| {
            let mut web_name_default: heapless::String<32> = heapless::String::new();
            web_name_default.push_str("google.com").unwrap();
            web_name_default
        });

    loop {
        if let Ok(addresses) = stack
            .dns_query(web_name.as_str(), embassy_net::dns::DnsQueryType::A)
            .await
        {
            defmt::info!("{}: {:?}", web_name.as_str(), addresses);
        } else {
            defmt::info!("{}: lookup failed", web_name.as_str());
        }
        embassy_time::Timer::after(embassy_time::Duration::from_secs(15)).await;
    }
}
