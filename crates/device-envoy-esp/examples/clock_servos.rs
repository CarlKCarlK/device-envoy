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

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use log::info;

use device_envoy_esp::{
    button::{ButtonWatch, ButtonWatchStatic, PressDuration, PressedTo},
    clock_sync::{h12_m_s, ClockSync, ClockSyncStatic, ONE_DAY, ONE_MINUTE, ONE_SECOND},
    flash_array::FlashArray,
    init_and_start,
    servo_player::{combine, linear, servo_player, AtEnd, ServoPlayer},
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto, WifiAutoEvent,
    },
    Error, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const FAST_MODE_SPEED: f32 = 720.0;
const CAPTIVE_PORTAL_SSID: &str = "EnvoyServoClock";

servo_player! {
    BottomServoPlayer {
        timer: Timer0,
        channel: Channel0,
        max_steps: 30,
    }
}

servo_player! {
    TopServoPlayer {
        timer: Timer1,
        channel: Channel1,
        max_steps: 30,
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

    let [wifi_auto_flash_block, timezone_flash_block] = FlashArray::<2>::new(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);

    let wifi_auto = WifiAuto::new(
        p.WIFI,
        wifi_auto_flash_block,
        p.GPIO6,
        PressedTo::Ground,
        CAPTIVE_PORTAL_SSID,
        [timezone_field],
        spawner,
    )?;

    let bottom_servo_player = BottomServoPlayer::new(&ledc, p.GPIO10, spawner)?;
    let top_servo_player = TopServoPlayer::new(&ledc, p.GPIO18, spawner)?;

    let servo_clock_display = ServoClockDisplay::new(bottom_servo_player, top_servo_player);
    let servo_clock_display_ref = &servo_clock_display;

    let (stack, button) = wifi_auto
        .connect(|wifi_auto_event| {
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

    static BUTTON_WATCH_STATIC: ButtonWatchStatic = ButtonWatch::new_static();
    let button_watch = ButtonWatch::new(&BUTTON_WATCH_STATIC, button, spawner)?;

    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();
    let clock_sync = ClockSync::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    );

    let mut state = State::HoursMinutes { speed: 1.0 };
    loop {
        state = match state {
            State::HoursMinutes { speed } => {
                state
                    .execute_hours_minutes(speed, &clock_sync, &button_watch, &servo_clock_display)
                    .await?
            }
            State::MinutesSeconds => {
                state
                    .execute_minutes_seconds(&clock_sync, &button_watch, &servo_clock_display)
                    .await?
            }
            State::EditOffset => {
                state
                    .execute_edit_offset(
                        &clock_sync,
                        &button_watch,
                        timezone_field,
                        &servo_clock_display,
                    )
                    .await?
            }
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    HoursMinutes { speed: f32 },
    MinutesSeconds,
    EditOffset,
}

impl State {
    async fn execute_hours_minutes(
        self,
        speed: f32,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatch<'_>,
        servo_clock_display: &ServoClockDisplay,
    ) -> Result<Self> {
        clock_sync.set_speed(speed).await;
        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        servo_clock_display.show_hours_minutes(hours, minutes).await;
        clock_sync.set_tick_interval(Some(ONE_MINUTE)).await;

        loop {
            match select(
                button_watch.wait_for_press_duration(),
                clock_sync.wait_for_tick(),
            )
            .await
            {
                Either::First(press_duration) => match (press_duration, speed.to_bits()) {
                    (PressDuration::Short, bits) if bits == 1.0f32.to_bits() => {
                        return Ok(Self::MinutesSeconds);
                    }
                    (PressDuration::Short, _) => {
                        return Ok(Self::HoursMinutes { speed: 1.0 });
                    }
                    (PressDuration::Long, _) => {
                        return Ok(Self::EditOffset);
                    }
                },
                Either::Second(tick) => {
                    let (hours, minutes, _) = h12_m_s(&tick.local_time);
                    servo_clock_display.show_hours_minutes(hours, minutes).await;
                }
            }
        }
    }

    async fn execute_minutes_seconds(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatch<'_>,
        servo_clock_display: &ServoClockDisplay,
    ) -> Result<Self> {
        clock_sync.set_speed(1.0).await;
        let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
        servo_clock_display.show_minutes_seconds(minutes, seconds);
        clock_sync.set_tick_interval(Some(ONE_SECOND)).await;

        loop {
            match select(
                button_watch.wait_for_press_duration(),
                clock_sync.wait_for_tick(),
            )
            .await
            {
                Either::First(PressDuration::Short) => {
                    return Ok(Self::HoursMinutes {
                        speed: FAST_MODE_SPEED,
                    });
                }
                Either::First(PressDuration::Long) => {
                    return Ok(Self::EditOffset);
                }
                Either::Second(tick) => {
                    let (_, minutes, seconds) = h12_m_s(&tick.local_time);
                    servo_clock_display.show_minutes_seconds(minutes, seconds);
                }
            }
        }
    }

    async fn execute_edit_offset(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatch<'_>,
        timezone_field: &TimezoneField,
        servo_clock_display: &ServoClockDisplay,
    ) -> Result<Self> {
        info!("Entering edit offset mode");
        clock_sync.set_speed(1.0).await;

        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        servo_clock_display
            .show_hours_minutes_indicator(hours, minutes)
            .await;

        const WIGGLE: [(u16, Duration); 2] = [
            (80, Duration::from_millis(250)),
            (100, Duration::from_millis(250)),
        ];
        servo_clock_display.bottom.animate(WIGGLE, AtEnd::Loop);

        let mut offset_minutes = clock_sync.offset_minutes();

        clock_sync.set_tick_interval(None).await;
        loop {
            match button_watch.wait_for_press_duration().await {
                PressDuration::Short => {
                    offset_minutes += 60;
                    const ONE_DAY_MINUTES: i32 = ONE_DAY.as_secs() as i32 / 60;
                    if offset_minutes >= ONE_DAY_MINUTES {
                        offset_minutes -= ONE_DAY_MINUTES;
                    }
                    clock_sync.set_offset_minutes(offset_minutes).await;

                    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
                    servo_clock_display
                        .show_hours_minutes_indicator(hours, minutes)
                        .await;
                    servo_clock_display.bottom.animate(WIGGLE, AtEnd::Loop);
                }
                PressDuration::Long => {
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("Saved timezone offset: {offset_minutes} minutes");
                    return Ok(Self::HoursMinutes { speed: 1.0 });
                }
            }
        }
    }
}

struct ServoClockDisplay {
    bottom: ServoPlayer,
    top: ServoPlayer,
}

impl ServoClockDisplay {
    fn new(bottom: ServoPlayer, top: ServoPlayer) -> Self {
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
