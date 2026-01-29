//! WifiAuto example with a custom website field for DNS lookups.

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

extern crate defmt_rtt as _;
extern crate panic_probe as _;

use device_kit::{
    Result,
    button::PressedTo,
    flash_array::{FlashArray, FlashArrayStatic},
    wifi_auto::{WifiAuto, WifiAutoEvent},
    wifi_auto::fields::{TextField, TextFieldStatic, TimezoneField, TimezoneFieldStatic},
};

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<core::convert::Infallible> {
    let p = embassy_rp::init(Default::default());

    static FLASH_STATIC: FlashArrayStatic = FlashArray::<3>::new_static();
    let [wifi_flash, website_flash, timezone_flash] = FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    static WEBSITE_STATIC: TextFieldStatic<32> = TextField::new_static();
    let website_field = TextField::new(
        &WEBSITE_STATIC,
        website_flash,
        "website",
        "Website",
        "google.com",
    );

    // Create timezone field
    static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash);

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
        [website_field, timezone_field], // Custom fields
        spawner,
    )?;

    let (stack, _button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    defmt::info!("Captive portal ready");
                }
                WifiAutoEvent::Connecting {
                    try_index,
                    try_count,
                } => {
                    defmt::info!(
                        "Connecting to WiFi (attempt {} of {})...",
                        try_index + 1,
                        try_count
                    );
                }
                WifiAutoEvent::ConnectionFailed => {
                    defmt::info!("WiFi connection failed");
                }
            }
            Ok(())
        })
        .await?;

    let website = website_field.text()?.unwrap_or_default();
    let offset_minutes = timezone_field.offset_minutes()?.unwrap_or(0);
    defmt::info!("Timezone offset minutes: {}", offset_minutes);

    loop {
        let query_name = website.as_str();
        if let Ok(addresses) = stack
            .dns_query(query_name, embassy_net::dns::DnsQueryType::A)
            .await
        {
            defmt::info!("{}: {:?}", query_name, addresses);
        } else {
            defmt::info!("{}: lookup failed", query_name);
        }

        embassy_time::Timer::after(embassy_time::Duration::from_secs(15)).await;
    }
}
