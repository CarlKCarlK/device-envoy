//! Wi-Fi enabled clock that visualizes time with two hobby servos.
//!
//! This example combines the `WifiAuto` captive-portal workflow with a servo-based
//! display. Because the servos are mounted reversed, the left servo shows minutes/seconds
//! and the right servo shows hours/minutes with 180-degree reflections applied.
//!
//! Hardware defaults:
//! - force-portal button on GPIO6 (wired to GND)
//! - bottom servo signal on GPIO10
//! - top servo signal on GPIO18

#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::{Infallible, TryFrom};

use device_envoy_example_common::clock_ui::{run_clock_ui, ClockUiEvent};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::PressedTo,
    button_watch,
    clock_sync::{ClockSyncEsp, ClockSyncStatic, ONE_MINUTE},
    flash_block::FlashBlockEsp,
    init_and_start,
    servo::Servo as _,
    servo::{combine, linear, servo_player, AtEnd, ServoPlayer as _, ServoPlayerHandle},
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto as _, WifiAutoEsp, WifiAutoEvent,
    },
    Error, Result,
};
esp_bootloader_esp_idf::esp_app_desc!();

const CAPTIVE_PORTAL_SSID: &str = "DeviceEnvoyClock";
const SERVO_MAX_STEPS: usize = 30;
type ClockServoPlayer = ServoPlayerHandle<SERVO_MAX_STEPS>;

button_watch! {
    ForcePortalButtonWatch {
        pin: GPIO6,
    }
}

servo_player! {
    BottomServoPlayer {
        pin: GPIO10,
        timer: Timer0,
        channel: Channel0,
        max_steps: SERVO_MAX_STEPS,
    }
}

servo_player! {
    TopServoPlayer {
        pin: GPIO18,
        timer: Timer1,
        channel: Channel1,
        max_steps: SERVO_MAX_STEPS,
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p, ledc: ledc);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    info!("Starting Wi-Fi servo clock (WifiAuto)");

    let [wifi_auto_flash_block, mut timezone_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);
    let button_watch6 = ForcePortalButtonWatch::new(p.GPIO6, PressedTo::Ground, spawner).await?;

    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        CAPTIVE_PORTAL_SSID,
        [timezone_field],
        spawner,
    )?;

    let bottom_servo_player = BottomServoPlayer::new(&ledc, p.GPIO10, spawner)?;
    let top_servo_player = TopServoPlayer::new(&ledc, p.GPIO18, spawner)?;

    let servo_clock_display = ServoClockDisplay::new(bottom_servo_player, top_servo_player);
    let servo_clock_display_ref = &servo_clock_display;

    let stack = wifi_auto
        .connect(&mut *button_watch6, |wifi_auto_event| {
            let servo_clock_display_ref = servo_clock_display_ref;
            async move {
                match wifi_auto_event {
                    WifiAutoEvent::CaptivePortalReady => {
                        info!("WifiAuto: setup mode ready");
                        servo_clock_display_ref.show_portal_ready();
                    }
                    WifiAutoEvent::Connecting { .. } => {
                        info!("WifiAuto: connecting");
                        servo_clock_display_ref.show_connecting();
                    }
                    WifiAutoEvent::ConnectionFailed => {
                        info!("WifiAuto: connection failed");
                        servo_clock_display_ref.show_connection_failed();
                    }
                }
                Ok(())
            }
        })
        .await?;

    info!("WiFi connected");
    servo_clock_display.show_portal_ready();

    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncEsp::new_static();
    let clock_sync = ClockSyncEsp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    run_clock_ui(
        &clock_sync,
        &mut *button_watch6,
        &mut timezone_flash_block,
        |clock_ui_event| {
            let servo_clock_display_ref = &servo_clock_display;
            async move {
                match clock_ui_event {
                    ClockUiEvent::RenderHoursMinutes { hours, minutes } => {
                        servo_clock_display_ref
                            .show_hours_minutes(hours, minutes)
                            .await;
                    }
                    ClockUiEvent::RenderMinutesSeconds { minutes, seconds } => {
                        servo_clock_display_ref.show_minutes_seconds(minutes, seconds);
                    }
                    ClockUiEvent::RenderHoursMinutesEdit { hours, minutes } => {
                        servo_clock_display_ref
                            .show_hours_minutes_indicator(hours, minutes)
                            .await;
                    }
                }
                Ok(())
            }
        },
    )
    .await
}

struct ServoClockDisplay {
    bottom: ClockServoPlayer,
    top: ClockServoPlayer,
}

impl ServoClockDisplay {
    fn new(bottom: ClockServoPlayer, top: ClockServoPlayer) -> Self {
        Self { bottom, top }
    }

    fn show_portal_ready(&self) {
        self.set_angles(90, 90);
    }

    fn show_connecting(&self) {
        const FIVE_SECONDS: Duration = Duration::from_secs(5);
        const PHASE1: [(u16, Duration); 10] = linear(180 - 18, 0, FIVE_SECONDS);
        const PHASE2: [(u16, Duration); 2] = linear(0, 180, FIVE_SECONDS);
        self.top
            .animate(combine::<10, 2, 12>(PHASE1, PHASE2), AtEnd::Loop);
        self.bottom
            .animate(combine::<2, 10, 12>(PHASE2, PHASE1), AtEnd::Loop);
    }

    fn show_connection_failed(&self) {
        self.set_angles(0, 180);
    }

    async fn show_hours_minutes(&self, hours: u8, minutes: u8) {
        let left_degrees = hours_to_degrees(hours);
        let right_degrees = sixty_to_degrees(minutes);
        self.set_angles(left_degrees, right_degrees);
        Timer::after(Duration::from_millis(500)).await;
        self.bottom.relax();
        self.top.relax();
    }

    async fn show_hours_minutes_indicator(&self, hours: u8, minutes: u8) {
        self.show_hours_minutes(hours, minutes).await;
    }

    fn show_minutes_seconds(&self, minutes: u8, seconds: u8) {
        let left_degrees = sixty_to_degrees(minutes);
        let right_degrees = sixty_to_degrees(seconds);
        self.set_angles(left_degrees, right_degrees);
    }

    fn set_angles(&self, left_degrees: i32, right_degrees: i32) {
        let physical_left = reflect_degrees(right_degrees);
        let physical_right = reflect_degrees(left_degrees);
        let left_u16 =
            u16::try_from(physical_left).expect("servo angles must be between 0 and 180 degrees");
        let right_u16 =
            u16::try_from(physical_right).expect("servo angles must be between 0 and 180 degrees");
        self.bottom.set_degrees(left_u16);
        self.top.set_degrees(right_u16);
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
