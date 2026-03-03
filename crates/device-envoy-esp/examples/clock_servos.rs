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
use esp_hal::gpio::DriveMode;
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::ledc::{channel::ChannelIFace, timer::TimerIFace};
use esp_hal::time::Rate;
use log::info;

use device_envoy_esp::{
    button::{ButtonWatch, ButtonWatchStatic, PressDuration, PressedTo},
    clock_sync::{h12_m_s, ClockSync, ClockSyncStatic, ONE_DAY, ONE_MINUTE, ONE_SECOND},
    flash_array::FlashArray,
    init_and_start,
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto, WifiAutoEvent,
    },
    Error, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

const FAST_MODE_SPEED: f32 = 720.0;
const CAPTIVE_PORTAL_SSID: &str = "EnvoyServoClock";

const SERVO_PERIOD_US: u32 = 20_000;
const SERVO_MIN_US: u32 = 500;
const SERVO_MAX_US: u32 = 2_500;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
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

    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut servo_timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    servo_timer
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })
        .expect("LEDC timer config failed");

    let mut bottom_servo = ledc.channel(channel::Number::Channel0, p.GPIO10);
    bottom_servo
        .configure(channel::config::Config {
            timer: &servo_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .expect("bottom servo channel config failed");

    let mut top_servo = ledc.channel(channel::Number::Channel1, p.GPIO18);
    top_servo
        .configure(channel::config::Config {
            timer: &servo_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .expect("top servo channel config failed");

    let mut servo_display = ServoClockDisplay::new(bottom_servo, top_servo);

    let (stack, button) = wifi_auto
        .connect(|wifi_auto_event| async move {
            match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => info!("WifiAuto: setup mode ready"),
                WifiAutoEvent::Connecting { .. } => info!("WifiAuto: connecting"),
                WifiAutoEvent::ConnectionFailed => info!("WifiAuto: connection failed"),
            }
            Ok(())
        })
        .await?;

    info!("WiFi connected");
    servo_display.show_portal_ready();

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
                    .execute_hours_minutes(speed, &clock_sync, &button_watch, &mut servo_display)
                    .await?
            }
            State::MinutesSeconds => {
                state
                    .execute_minutes_seconds(&clock_sync, &button_watch, &mut servo_display)
                    .await?
            }
            State::EditOffset => {
                state
                    .execute_edit_offset(
                        &clock_sync,
                        &button_watch,
                        timezone_field,
                        &mut servo_display,
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
        servo_display: &mut ServoClockDisplay<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(speed).await;
        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        servo_display.show_hours_minutes(hours, minutes).await;
        clock_sync.set_tick_interval(Some(ONE_MINUTE)).await;

        loop {
            match select(
                button_watch.wait_for_press_duration(),
                clock_sync.wait_for_tick(),
            )
            .await
            {
                Either::First(press_duration) => match (press_duration, speed.to_bits()) {
                    (PressDuration::Short, bits) if bits == 1.0_f32.to_bits() => {
                        return Ok(Self::MinutesSeconds);
                    }
                    (PressDuration::Short, _) => {
                        return Ok(Self::HoursMinutes { speed: 1.0 });
                    }
                    (PressDuration::Long, _) => {
                        return Ok(Self::EditOffset);
                    }
                },
                Either::Second(clock_sync_tick) => {
                    let (hours, minutes, _) = h12_m_s(&clock_sync_tick.local_time);
                    servo_display.show_hours_minutes(hours, minutes).await;
                }
            }
        }
    }

    async fn execute_minutes_seconds(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatch<'_>,
        servo_display: &mut ServoClockDisplay<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(1.0).await;
        let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
        servo_display.show_minutes_seconds(minutes, seconds);
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
                Either::Second(clock_sync_tick) => {
                    let (_, minutes, seconds) = h12_m_s(&clock_sync_tick.local_time);
                    servo_display.show_minutes_seconds(minutes, seconds);
                }
            }
        }
    }

    async fn execute_edit_offset(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatch<'_>,
        timezone_field: &TimezoneField,
        servo_display: &mut ServoClockDisplay<'_>,
    ) -> Result<Self> {
        info!("Entering edit offset mode");
        clock_sync.set_speed(1.0).await;

        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        servo_display.show_hours_minutes(hours, minutes).await;

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
                    servo_display.show_hours_minutes(hours, minutes).await;
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

struct ServoClockDisplay<'a> {
    bottom_servo: channel::Channel<'a, LowSpeed>,
    top_servo: channel::Channel<'a, LowSpeed>,
}

impl<'a> ServoClockDisplay<'a> {
    fn new(
        bottom_servo: channel::Channel<'a, LowSpeed>,
        top_servo: channel::Channel<'a, LowSpeed>,
    ) -> Self {
        Self {
            bottom_servo,
            top_servo,
        }
    }

    fn show_portal_ready(&mut self) {
        self.set_angles(90, 90);
    }

    async fn show_hours_minutes(&mut self, hours: u8, minutes: u8) {
        let left_degrees = hours_to_degrees(hours);
        let right_degrees = sixty_to_degrees(minutes);
        self.set_angles(left_degrees, right_degrees);
        Timer::after(Duration::from_millis(500)).await;
        self.relax();
    }

    fn show_minutes_seconds(&mut self, minutes: u8, seconds: u8) {
        let left_degrees = sixty_to_degrees(minutes);
        let right_degrees = sixty_to_degrees(seconds);
        self.set_angles(left_degrees, right_degrees);
    }

    fn set_angles(&mut self, left_degrees: i32, right_degrees: i32) {
        let physical_left = reflect_degrees(right_degrees);
        let physical_right = reflect_degrees(left_degrees);

        let left_u16 = u16::try_from(physical_left).expect("left angle must be between 0 and 180");
        let right_u16 =
            u16::try_from(physical_right).expect("right angle must be between 0 and 180");

        write_degrees(&mut self.bottom_servo, left_u16);
        write_degrees(&mut self.top_servo, right_u16);
    }

    fn relax(&mut self) {
        self.bottom_servo.set_duty(0).expect("bottom relax duty");
        self.top_servo.set_duty(0).expect("top relax duty");
    }
}

fn write_degrees(servo: &mut channel::Channel<'_, LowSpeed>, degrees: u16) {
    assert!(degrees <= 180);
    let pulse_span = SERVO_MAX_US - SERVO_MIN_US;
    let pulse_us = SERVO_MIN_US + (u32::from(degrees) * pulse_span + 90) / 180;
    let duty_pct = ((pulse_us * 100) + (SERVO_PERIOD_US / 2)) / SERVO_PERIOD_US;
    let duty_pct = u8::try_from(duty_pct).expect("duty percent fits in u8");
    servo.set_duty(duty_pct).expect("servo duty update");
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
