#![allow(missing_docs)]
//! Wi-Fi enabled clock that visualizes time with two servos.
//!
//! This example combines the `WifiAutoRp` captive-portal workflow with a servo-based
//! display. Because the servos are mounted reversed, the left servo shows minutes/seconds
//! and the right servo shows hours/minutes with 180° reflections applied.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::{Infallible, TryFrom};
use defmt::info;
use defmt_rtt as _;
use device_envoy_example_common::clock_ui::{ClockUiEvent, run_clock_ui};
use device_envoy_rp::button::PressedTo;
use device_envoy_rp::button_watch;
use device_envoy_rp::clock_sync::{ClockSyncRp, ClockSyncStatic, ONE_MINUTE};
use device_envoy_rp::flash_block::FlashBlockRp;
use device_envoy_rp::wifi_auto::fields::{TimezoneField, TimezoneFieldStatic};
use device_envoy_rp::wifi_auto::{WifiAutoEvent, WifiAutoRp};
use device_envoy_rp::{Error, Result};
use device_envoy_rp::{
    servo::Servo as _,
    servo::{AtEnd, ServoPlayer as _, combine, linear, servo_player},
};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use panic_probe as _;

button_watch! {
    ButtonWatch13 {
        pin: PIN_13,
    }
}

// Define two typed servo players at module scope
servo_player! {
    BottomServoPlayer {
        pin: PIN_11,
        max_steps: 30,
    }
}

servo_player! {
    TopServoPlayer {
        pin: PIN_12,
        max_steps: 30,
    }
}

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting Wi-Fi servo clock (WifiAutoRp)");
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
        "PicoServoClock", // Captive-portal SSID
        [timezone_field],
        spawner,
    )?;

    // Configure two servos for the display.
    let bottom_servo = BottomServoPlayer::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
    let top_servo = TopServoPlayer::new(p.PIN_12, p.PWM_SLICE6, spawner)?;
    let servo_display = ServoClockDisplay::new(bottom_servo, top_servo);

    // Connect Wi-Fi, using the servos for status indications.
    let servo_display_ref = &servo_display;
    // TODO00 verify startup ButtonWatch13 behavior still matches reset-button expectations.
    let stack = wifi_auto
        .connect(&mut *button_watch13, |event| {
            let servo_display_ref = servo_display_ref;
            async move {
                match event {
                    WifiAutoEvent::CaptivePortalReady => {
                        servo_display_ref.show_portal_ready().await;
                    }
                    WifiAutoEvent::Connecting { .. } => servo_display_ref.show_connecting().await,
                    WifiAutoEvent::ConnectionFailed => {
                        // No-op; portal remains visible on failure.
                    }
                }
                Ok(())
            }
        })
        .await?;

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

    let servo_display_ref = &servo_display;
    run_clock_ui(
        &clock_sync,
        &mut *button_watch13,
        |clock_ui_event| async move {
            match clock_ui_event {
                ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                    servo_display_ref.show_hours_minutes(hours, minutes).await;
                }
                ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                    servo_display_ref
                        .show_minutes_seconds(minutes, seconds)
                        .await;
                }
                ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                    servo_display_ref
                        .show_hours_minutes_indicator(hours, minutes)
                        .await;
                }
                ClockUiEvent::OffsetPersistRequested { offset_minutes } => {
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("Offset saved to flash: {} minutes", offset_minutes);
                }
            }
            Ok(())
        },
    )
    .await
}

struct ServoClockDisplay {
    bottom: &'static BottomServoPlayer,
    top: &'static TopServoPlayer,
}

impl ServoClockDisplay {
    fn new(bottom: &'static BottomServoPlayer, top: &'static TopServoPlayer) -> Self {
        Self { bottom, top }
    }

    async fn show_portal_ready(&self) {
        self.bottom.set_degrees(90);
        self.top.set_degrees(90);
    }

    async fn show_connecting(&self) {
        // Animate both servos in complementary two-phase sweeps.
        const FIVE_SECONDS: Duration = Duration::from_secs(5);
        const PHASE1: [(u16, Duration); 10] = linear(180 - 18, 0, FIVE_SECONDS);
        const PHASE2: [(u16, Duration); 2] = linear(0, 180, FIVE_SECONDS);
        self.top.animate(combine!(PHASE1, PHASE2), AtEnd::Loop);
        self.bottom.animate(combine!(PHASE2, PHASE1), AtEnd::Loop);
    }

    async fn show_hours_minutes(&self, hours: u8, minutes: u8) {
        let left_angle = hours_to_degrees(hours);
        let right_angle = sixty_to_degrees(minutes);
        self.set_angles(left_angle, right_angle).await;
        Timer::after(Duration::from_millis(500)).await;
        self.bottom.relax();
        self.top.relax();
    }

    async fn show_hours_minutes_indicator(&self, hours: u8, minutes: u8) {
        let left_angle = hours_to_degrees(hours);
        let right_angle = sixty_to_degrees(minutes);
        self.set_angles(left_angle, right_angle).await;
        Timer::after(Duration::from_millis(500)).await;
        self.bottom.relax();
        self.top.relax();
    }

    async fn show_minutes_seconds(&self, minutes: u8, seconds: u8) {
        let left_angle = sixty_to_degrees(minutes);
        let right_angle = sixty_to_degrees(seconds);
        self.set_angles(left_angle, right_angle).await;
    }

    async fn set_angles(&self, left_degrees: i32, right_degrees: i32) {
        // Swap servos and reflect angles for physical orientation.
        let physical_left = reflect_degrees(right_degrees);
        let physical_right = reflect_degrees(left_degrees);
        let left_angle =
            u16::try_from(physical_left).expect("servo angles must be between 0 and 180 degrees");
        let right_angle =
            u16::try_from(physical_right).expect("servo angles must be between 0 and 180 degrees");
        self.bottom.set_degrees(left_angle);
        self.top.set_degrees(right_angle);
    }
}

#[inline]
fn hours_to_degrees(hours: u8) -> i32 {
    assert!((1..=12).contains(&hours));
    let normalized_hour = hours % 12;
    i32::from(normalized_hour) * 180 / 12
}

#[inline]
fn sixty_to_degrees(value: u8) -> i32 {
    assert!(value < 60);
    i32::from(value) * 180 / 60
}

#[inline]
fn reflect_degrees(degrees: i32) -> i32 {
    assert!((0..=180).contains(&degrees));
    180 - degrees
}
