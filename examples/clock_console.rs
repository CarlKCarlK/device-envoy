#![allow(missing_docs)]
//! Console Clock - WiFi-synced time logging to console
//!
//! This example demonstrates WiFi connection with auto-provisioning
//! and logs time sync events to the console.

#![allow(missing_docs)]
#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;
use defmt::info;
use defmt_rtt as _;
use device_kit::button::PressedTo;
use device_kit::clock_sync::{ClockSync, ClockSyncStatic, ONE_SECOND};
use device_kit::flash_array::FlashArray;
use device_kit::wifi_auto::WifiAuto;
use device_kit::wifi_auto::WifiAutoEvent;
use device_kit::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_kit::{Error, Result};
use embassy_executor::Spawner;
use panic_probe as _;

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Console Clock with WiFi");

    // Initialize RP2040 peripherals
    let p = embassy_rp::init(Default::default());

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, timezone_flash_block] = FlashArray::<2>::new(p.FLASH)?;

    // Define timezone field for captive portal
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up WiFi via captive portal
    let wifi_auto = WifiAuto::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data pin
        p.PIO0,    // CYW43 PIO interface
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        p.PIN_13, // Reset button pin
        PressedTo::Ground,
        "www.picoclock.net",
        [timezone_field],
        spawner,
    )?;

    // Connect to WiFi
    let (stack, _button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    info!("Captive portal ready - connect to WiFi network");
                }
                WifiAutoEvent::Connecting {
                    try_index,
                    try_count,
                } => {
                    info!(
                        "Connecting to WiFi (attempt {} of {})...",
                        try_index + 1,
                        try_count
                    );
                }
                WifiAutoEvent::ConnectionFailed => {
                    info!("WiFi connection failed!");
                }
            }
            Ok(())
        })
        .await?;

    info!("WiFi connected successfully!");

    // Create ClockSync device with timezone from WiFi portal
    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();
    let clock_sync = ClockSync::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_SECOND),
        spawner,
    );

    info!("WiFi connected, entering event loop");

    // Main event loop - log time on every tick
    loop {
        let tick = clock_sync.wait_for_tick().await;
        let time_info = tick.local_time;
        info!(
            "Current time: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            time_info.year(),
            u8::from(time_info.month()),
            time_info.day(),
            time_info.hour(),
            time_info.minute(),
            time_info.second(),
        );
    }
}
