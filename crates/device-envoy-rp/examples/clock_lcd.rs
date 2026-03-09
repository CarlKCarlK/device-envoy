#![allow(missing_docs)]
//! LCD Clock - Event-driven time display with WiFi sync

#![cfg(feature = "wifi")]
#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, fmt};
use defmt::*;
use defmt_rtt as _;
use device_envoy_rp::button::PressedTo;
use device_envoy_rp::clock_sync::{ClockSync as _, ClockSyncRp, ClockSyncStatic, ONE_SECOND};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::i2cs;
use device_envoy_rp::lcd_text::LcdText as _;
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::wifi_auto::{WifiAuto as _, WifiAutoRp};
use device_envoy_rp::{Error, Result};
use embassy_executor::Spawner;
use heapless::String;
use panic_probe as _;

i2cs! {
    i2c: I2C0,
    sda_pin: PIN_4,
    scl_pin: PIN_5,
    LcdTexts0 {
        LcdTextClock { width: 16, height: 2, address: 0x27 },
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
    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define timezone field for captive portal
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up WiFi via captive portal
    let wifi_auto = WifiAutoRp::new(
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
    let lcd_text_ref = lcd_text;
    let (stack, _button) = wifi_auto
        .connect(|wifi_auto_event| {
            let lcd_text_ref = lcd_text_ref;
            async move {
                match wifi_auto_event {
                    device_envoy_rp::wifi_auto::WifiAutoEvent::CaptivePortalReady => {
                        lcd_text_ref.write_text("Join WiFi:\nwww.picoclock.net");
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
        lcd_text.write_text(text.as_str());
    }
}
