#![allow(missing_docs)]
//! LCD Clock - Event-driven time display with WiFi sync

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, fmt};
use defmt::*;
use defmt_rtt as _;
use device_envoy::button::PressedTo;
use device_envoy::char_lcd::{CharLcd, CharLcdStatic};
use device_envoy::clock_sync::{ClockSync, ClockSyncStatic, ONE_SECOND};
use device_envoy::flash_array::FlashArray;
use device_envoy::wifi_auto::WifiAuto;
use device_envoy::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy::{Error, Result};
use embassy_executor::Spawner;
use heapless::String;
use panic_probe as _;

// ============================================================================
// Main Orchestrator
// ============================================================================

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    // If it returns, something went wrong.
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting LCD Clock with WiFi");

    // Initialize RP2040 peripherals
    let p = embassy_rp::init(Default::default());

    // Initialize CharLcd
    static CHAR_LCD_STATIC: CharLcdStatic = CharLcd::new_static();
    let char_lcd = CharLcd::new(&CHAR_LCD_STATIC, p.I2C0, p.PIN_5, p.PIN_4, spawner)?;

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
    let (stack, _button) = wifi_auto.connect(|_event| async move { Ok(()) }).await?;

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

    info!("Entering main event loop");

    // Main orchestrator loop - owns LCD and displays the clock
    loop {
        let tick = clock_sync.wait_for_tick().await;
        let time_info = tick.local_time;
        let mut text = String::<64>::new();
        let (hour12, am_pm) = if time_info.hour() == 0 {
            (12, "AM")
        } else if time_info.hour() < 12 {
            (time_info.hour(), "AM")
        } else if time_info.hour() == 12 {
            (12, "PM")
        } else {
            #[expect(clippy::arithmetic_side_effects, reason = "hour guaranteed 13-23")]
            {
                (time_info.hour() - 12, "PM")
            }
        };
        fmt::Write::write_fmt(
            &mut text,
            format_args!(
                "{:2}:{:02}:{:02} {}\n{:04}-{:02}-{:02}",
                hour12,
                time_info.minute(),
                time_info.second(),
                am_pm,
                time_info.year(),
                u8::from(time_info.month()),
                time_info.day()
            ),
        )
        .map_err(|_| Error::FormatError)?;
        char_lcd.write_text(text, 0).await;
    }
}
