#![allow(missing_docs)]
//! LCD Clock - Event-driven time display with WiFi sync

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, fmt};
use defmt::*;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::button::PressedTo;
use device_envoy_rp::button_watch;
use device_envoy_rp::clock_sync::{ClockSyncRp, ClockSyncStatic, ONE_SECOND};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::i2cs;
use device_envoy_rp::lcd_text::LcdText as _;
use device_envoy_rp::wifi_auto::WifiAutoRp;
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::{Error, Result};
use embassy_executor::Spawner;
use panic_probe as _;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTexts0 {
        LcdTextClock { width: 16, height: 2, address: 0x27 },
    }
}

button_watch! {
    ButtonWatch13 {
        pin: PIN_13,
    }
}

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

    // Initialize LcdText
    let (lcd_text,) = LcdTexts0::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
    lcd_text.write_text("Booting...\nLCD Clock");

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, mut timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define timezone field for captive portal
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up WiFi via captive portal
    let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data pin
        p.PIO0,    // CYW43 PIO interface
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        "DeviceEnvoyClock",
        [timezone_field],
        spawner,
    )?;

    // Connect to WiFi
    let lcd_text_ref = lcd_text;
    let stack = wifi_auto
        .connect(&mut *button_watch13, |wifi_auto_event| {
            let lcd_text_ref = lcd_text_ref;
            async move {
                match wifi_auto_event {
                    device_envoy_rp::wifi_auto::WifiAutoEvent::CaptivePortalReady => {
                        lcd_text_ref.write_text("Join WiFi:\nDeviceEnvoyClock");
                    }
                    device_envoy_rp::wifi_auto::WifiAutoEvent::Connecting { .. } => {
                        lcd_text_ref.write_text("Connecting...\nPlease wait");
                    }
                    device_envoy_rp::wifi_auto::WifiAutoEvent::ConnectionFailed => {
                        lcd_text_ref.write_text("WiFi failed\nRetry setup");
                    }
                }
                Ok(())
            }
        })
        .await?;

    // Create ClockSync device with timezone from WiFi portal
    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_SECOND),
        spawner,
    );

    info!("Entering main event loop");
    lcd_text.write_text("WiFi connected\nWaiting NTP");

    let lcd_text_ref = lcd_text;
    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        &mut timezone_flash_block,
        |clock_ui_event| async move {
            let text = event_text(clock_ui_event).map_err(|_| Error::FormatError)?;
            lcd_text_ref.write_text(text.as_str());
            Ok(())
        },
    )
    .await
}

fn event_text(clock_ui_event: ClockUiEvent) -> Result<heapless::String<32>, fmt::Error> {
    let mut text = heapless::String::<32>::new();
    match clock_ui_event {
        ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
            fmt::Write::write_fmt(
                &mut text,
                format_args!("{:>2}:{:02} HH:MM\nshort for MM:SS", hours, minutes),
            )?;
        }
        ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
            fmt::Write::write_fmt(
                &mut text,
                format_args!("{:02}:{:02} MM:SS\nshort for FAST", minutes, seconds),
            )?;
        }
        ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
            fmt::Write::write_fmt(
                &mut text,
                format_args!("{:>2}:{:02} TZ EDIT\nshort +1h long OK", hours, minutes),
            )?;
        }
    }
    Ok(text)
}
