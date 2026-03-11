#![allow(missing_docs)]
//! Wi-Fi enabled 4-digit clock that provisions credentials through `WifiAutoRp`.
//!
//! This example demonstrates how to pair the shared captive-portal workflow with the
//! `ClockLed4` state machine. The `WifiAutoRp` helper owns Wi-Fi onboarding while the
//! clock display reflects progress and, once connected, continues handling user input.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;
use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::button::PressedTo;
use device_envoy_rp::button_watch;
use device_envoy_rp::clock_sync::{ClockSyncRp, ClockSyncStatic, ONE_MINUTE};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::led4::{
    BlinkState, Led4 as _, Led4Rp, Led4RpStatic, OutputArray, circular_outline_animation,
};
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::wifi_auto::{WifiAutoEvent, WifiAutoRp};
use device_envoy_rp::{Error, Result};
use embassy_executor::Spawner;
use embassy_rp::gpio::{self, Level};
use panic_probe as _;

button_watch! {
    ButtonWatch13 {
        pin: PIN_13,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Wi-Fi 4-digit clock (WifiAutoRp)");
    let p = embassy_rp::init(Default::default());

    // Use two blocks of flash storage: Wi-Fi credentials + timezone
    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    // Define HTML to ask for timezone on the captive portal.
    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    // Set up Wifi via a captive portal. The button pin is used to reset stored credentials.
    let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,  // CYW43 power
        p.PIN_24,  // CYW43 clock
        p.PIN_25,  // CYW43 chip select
        p.PIN_29,  // CYW43 data pin
        p.PIO0,    // CYW43 PIO interface
        p.DMA_CH0, // CYW43 DMA channel
        wifi_credentials_flash_block,
        "www.picoclock.net", // Captive-portal SSID
        [timezone_field],    // Custom fields to ask for
        spawner,
    )?;

    // Set up the LED4 display.
    let cell_pins = OutputArray::new([
        gpio::Output::new(p.PIN_1, Level::High),
        gpio::Output::new(p.PIN_2, Level::High),
        gpio::Output::new(p.PIN_3, Level::High),
        gpio::Output::new(p.PIN_4, Level::High),
    ]);

    let segment_pins = OutputArray::new([
        gpio::Output::new(p.PIN_5, Level::Low),
        gpio::Output::new(p.PIN_6, Level::Low),
        gpio::Output::new(p.PIN_7, Level::Low),
        gpio::Output::new(p.PIN_8, Level::Low),
        gpio::Output::new(p.PIN_9, Level::Low),
        gpio::Output::new(p.PIN_10, Level::Low),
        gpio::Output::new(p.PIN_11, Level::Low),
        gpio::Output::new(p.PIN_12, Level::Low),
    ]);

    static LED4_STATIC: Led4RpStatic = Led4Rp::new_static();
    let led4 = Led4Rp::new(&LED4_STATIC, cell_pins, segment_pins, spawner)?;

    // Connect Wi-Fi, using the clock display for status.
    let led4_ref = &led4;
    // TODO00 verify startup ButtonWatch13 behavior still matches reset-button expectations.
    let stack = wifi_auto
        .connect(&mut *button_watch13, |event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    led4_ref.write_text(['j', 'o', 'i', 'n'], BlinkState::BlinkingAndOn);
                }
                WifiAutoEvent::Connecting { .. } => {
                    led4_ref.animate_text(circular_outline_animation(true));
                }
                WifiAutoEvent::ConnectionFailed => {
                    led4_ref.write_text(['F', 'A', 'I', 'L'], BlinkState::BlinkingButOff);
                }
            }
            Ok(())
        })
        .await?;

    led4.write_text(['D', 'O', 'N', 'E'], BlinkState::Solid);
    info!("WiFi connected");

    // Read the timezone offset, an extra field that WiFi portal saved to flash.
    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    // Create a ClockSync device that knows its timezone offset.
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    let led4_ref = &led4;
    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        |clock_ui_event| async move {
            match clock_ui_event {
                ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                    led4_ref.write_text(hours_minutes_text(hours, minutes), BlinkState::Solid);
                }
                ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                    led4_ref.write_text(minutes_seconds_text(minutes, seconds), BlinkState::Solid);
                }
                ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                    led4_ref.write_text(
                        hours_minutes_text(hours, minutes),
                        BlinkState::BlinkingAndOn,
                    );
                }
                ClockUiEvent::OffsetPersistRequested { offset_minutes } => {
                    timezone_field.set_offset_minutes(offset_minutes)?;
                }
            }
            Ok(())
        },
    )
    .await
}

fn hours_minutes_text(hours: u8, minutes: u8) -> [char; 4] {
    [
        tens_hours(hours),
        ones_digit(hours),
        tens_digit(minutes),
        ones_digit(minutes),
    ]
}

fn minutes_seconds_text(minutes: u8, seconds: u8) -> [char; 4] {
    [
        tens_digit(minutes),
        ones_digit(minutes),
        tens_digit(seconds),
        ones_digit(seconds),
    ]
}

#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    reason = "Value < 60 ensures division is safe"
)]
const fn tens_digit(value: u8) -> char {
    ((value / 10) + b'0') as char
}

#[inline]
const fn tens_hours(value: u8) -> char {
    if value >= 10 { '1' } else { ' ' }
}

#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    reason = "Value < 60 ensures division is safe"
)]
const fn ones_digit(value: u8) -> char {
    ((value % 10) + b'0') as char
}
