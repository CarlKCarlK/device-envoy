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

use core::borrow::Borrow;
use core::convert::{Infallible, TryFrom};

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::DriveMode;
use esp_hal::ledc::{channel, timer, LowSpeed};
use esp_hal::ledc::{channel::ChannelIFace, timer::TimerIFace};
use esp_hal::time::Rate;
use heapless::Vec;
use log::info;
use static_cell::StaticCell;

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
const SERVO_MIN_US_DEFAULT: u32 = 500;
const SERVO_MAX_US_DEFAULT: u32 = 2_500;
const SERVO_MAX_DEGREES_DEFAULT: u16 = 180;

const SERVO_MAX_STEPS: usize = 30;

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

    static BOTTOM_SERVO_PLAYER_STATIC: ServoPlayerStatic = ServoPlayer::new_static();
    let bottom_servo_player = ServoPlayer::new(
        &BOTTOM_SERVO_PLAYER_STATIC,
        &ledc,
        p.GPIO10,
        timer::Number::Timer0,
        channel::Number::Channel0,
        SERVO_MIN_US_DEFAULT,
        SERVO_MAX_US_DEFAULT,
        SERVO_MAX_DEGREES_DEFAULT,
        spawner,
    )?;

    static TOP_SERVO_PLAYER_STATIC: ServoPlayerStatic = ServoPlayer::new_static();
    let top_servo_player = ServoPlayer::new(
        &TOP_SERVO_PLAYER_STATIC,
        &ledc,
        p.GPIO18,
        timer::Number::Timer1,
        channel::Number::Channel1,
        SERVO_MIN_US_DEFAULT,
        SERVO_MAX_US_DEFAULT,
        SERVO_MAX_DEGREES_DEFAULT,
        spawner,
    )?;

    let servo_clock_display = ServoClockDisplay::new(bottom_servo_player, top_servo_player);
    let servo_clock_display_ref = &servo_clock_display;

    let (stack, button) = wifi_auto
        .connect(|wifi_auto_event| {
            let servo_clock_display_ref = servo_clock_display_ref;
            async move {
                match wifi_auto_event {
                    WifiAutoEvent::CaptivePortalReady => {
                        info!("WifiAuto: setup mode ready");
                        servo_clock_display_ref.show_portal_ready()?;
                    }
                    WifiAutoEvent::Connecting { .. } => {
                        info!("WifiAuto: connecting");
                        servo_clock_display_ref.show_connecting();
                    }
                    WifiAutoEvent::ConnectionFailed => {
                        info!("WifiAuto: connection failed");
                        servo_clock_display_ref.show_connection_failed()?;
                    }
                }
                Ok(())
            }
        })
        .await?;

    info!("WiFi connected");
    servo_clock_display.show_portal_ready()?;

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
        servo_clock_display
            .show_hours_minutes(hours, minutes)
            .await?;
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
                    servo_clock_display
                        .show_hours_minutes(hours, minutes)
                        .await?;
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
        servo_clock_display.show_minutes_seconds(minutes, seconds)?;
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
                    servo_clock_display.show_minutes_seconds(minutes, seconds)?;
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
            .await?;

        const WIGGLE: [(u16, Duration); 2] = [
            (80, Duration::from_millis(250)),
            (100, Duration::from_millis(250)),
        ];
        servo_clock_display
            .bottom_servo_player
            .animate(WIGGLE, AtEnd::Loop);

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
                        .await?;
                    servo_clock_display
                        .bottom_servo_player
                        .animate(WIGGLE, AtEnd::Loop);
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
    bottom_servo_player: ServoPlayer,
    top_servo_player: ServoPlayer,
}

impl ServoClockDisplay {
    fn new(bottom_servo_player: ServoPlayer, top_servo_player: ServoPlayer) -> Self {
        Self {
            bottom_servo_player,
            top_servo_player,
        }
    }

    fn show_portal_ready(&self) -> Result<()> {
        self.set_angles(90, 90)
    }

    fn show_connecting(&self) {
        const FIVE_SECONDS: Duration = Duration::from_secs(5);
        const PHASE1: [(u16, Duration); 10] = linear(180 - 18, 0, FIVE_SECONDS);
        const PHASE2: [(u16, Duration); 2] = linear(0, 180, FIVE_SECONDS);
        self.top_servo_player
            .animate(combine::<10, 2, 12>(PHASE1, PHASE2), AtEnd::Loop);
        self.bottom_servo_player
            .animate(combine::<2, 10, 12>(PHASE2, PHASE1), AtEnd::Loop);
    }

    fn show_connection_failed(&self) -> Result<()> {
        self.set_angles(0, 180)
    }

    async fn show_hours_minutes(&self, hours: u8, minutes: u8) -> Result<()> {
        let left_degrees = hours_to_degrees(hours);
        let right_degrees = sixty_to_degrees(minutes);
        self.set_angles(left_degrees, right_degrees)?;
        Timer::after(Duration::from_millis(500)).await;
        self.bottom_servo_player.relax();
        self.top_servo_player.relax();
        Ok(())
    }

    async fn show_hours_minutes_indicator(&self, hours: u8, minutes: u8) -> Result<()> {
        self.show_hours_minutes(hours, minutes).await
    }

    fn show_minutes_seconds(&self, minutes: u8, seconds: u8) -> Result<()> {
        let left_degrees = sixty_to_degrees(minutes);
        let right_degrees = sixty_to_degrees(seconds);
        self.set_angles(left_degrees, right_degrees)
    }

    fn set_angles(&self, left_degrees: i32, right_degrees: i32) -> Result<()> {
        let physical_left = reflect_degrees(right_degrees);
        let physical_right = reflect_degrees(left_degrees);

        let left_u16 = u16::try_from(physical_left).map_err(|_| Error::FormatError)?;
        let right_u16 = u16::try_from(physical_right).map_err(|_| Error::FormatError)?;

        self.bottom_servo_player.set_degrees(left_u16);
        self.top_servo_player.set_degrees(right_u16);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AtEnd {
    Loop,
    Hold,
    Relax,
}

enum PlayerCommand {
    Set {
        degrees: u16,
    },
    Animate {
        steps: Vec<(u16, Duration), SERVO_MAX_STEPS>,
        mode: AtEnd,
    },
    Hold,
    Relax,
}

type PlayerCommandSignal = Signal<CriticalSectionRawMutex, PlayerCommand>;

struct ServoPlayerStatic {
    timer: StaticCell<timer::Timer<'static, LowSpeed>>,
    channel: StaticCell<channel::Channel<'static, LowSpeed>>,
    command: PlayerCommandSignal,
}

#[derive(Clone, Copy)]
struct ServoPlayer {
    command: &'static PlayerCommandSignal,
}

impl ServoPlayer {
    const fn new_static() -> ServoPlayerStatic {
        ServoPlayerStatic {
            timer: StaticCell::new(),
            channel: StaticCell::new(),
            command: Signal::new(),
        }
    }

    fn new(
        servo_player_static: &'static ServoPlayerStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        timer_number: timer::Number,
        channel_number: channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
        spawner: Spawner,
    ) -> Result<Self> {
        let mut servo = Servo::new(
            servo_player_static,
            ledc,
            pin,
            timer_number,
            channel_number,
            min_us,
            max_us,
            max_degrees,
        )?;

        servo.set_degrees(0)?;

        spawner
            .spawn(servo_player_loop(servo, &servo_player_static.command))
            .map_err(Error::TaskSpawn)?;

        Ok(Self {
            command: &servo_player_static.command,
        })
    }

    fn set_degrees(&self, degrees: u16) {
        self.command.signal(PlayerCommand::Set { degrees });
    }

    fn hold(&self) {
        self.command.signal(PlayerCommand::Hold);
    }

    fn relax(&self) {
        self.command.signal(PlayerCommand::Relax);
    }

    fn animate<I>(&self, steps: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<(u16, Duration)>,
    {
        let mut sequence: Vec<(u16, Duration), SERVO_MAX_STEPS> = Vec::new();
        for step in steps {
            let step = *step.borrow();
            assert!(step.1.as_micros() > 0);
            sequence.push(step).expect("animation sequence fits");
        }
        assert!(!sequence.is_empty());

        self.command.signal(PlayerCommand::Animate {
            steps: sequence,
            mode: at_end,
        });
    }
}

#[embassy_executor::task]
async fn servo_player_loop(mut servo: Servo, command: &'static PlayerCommandSignal) -> ! {
    let mut current_degrees: u16 = 0;
    loop {
        match command.wait().await {
            PlayerCommand::Set { degrees } => {
                if servo.set_degrees(degrees).is_ok() {
                    current_degrees = degrees;
                }
            }
            PlayerCommand::Hold => {
                let _ = servo.hold();
            }
            PlayerCommand::Relax => {
                let _ = servo.relax();
            }
            PlayerCommand::Animate { steps, mode } => {
                run_animation(&mut servo, command, &steps, mode, &mut current_degrees).await;
            }
        }
    }
}

async fn run_animation(
    servo: &mut Servo,
    command: &'static PlayerCommandSignal,
    steps: &[(u16, Duration)],
    mode: AtEnd,
    current_degrees: &mut u16,
) {
    loop {
        for step in steps {
            if *current_degrees != step.0 {
                if servo.set_degrees(step.0).is_ok() {
                    *current_degrees = step.0;
                }
            }

            match select(Timer::after(step.1), command.wait()).await {
                Either::First(_) => {}
                Either::Second(new_command) => {
                    command.signal(new_command);
                    return;
                }
            }
        }

        match mode {
            AtEnd::Loop => {}
            AtEnd::Hold => return,
            AtEnd::Relax => {
                let _ = servo.relax();
                return;
            }
        }
    }
}

struct Servo {
    channel: &'static mut channel::Channel<'static, LowSpeed>,
    min_us: u32,
    max_us: u32,
    max_degrees: u16,
}

impl Servo {
    fn new(
        servo_player_static: &'static ServoPlayerStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        timer_number: timer::Number,
        channel_number: channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
    ) -> Result<Self> {
        assert!(min_us < max_us);
        assert!(max_degrees > 0);

        let timer = servo_player_static
            .timer
            .init(ledc.timer::<LowSpeed>(timer_number));
        timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })?;

        let channel = servo_player_static
            .channel
            .init(ledc.channel(channel_number, pin));
        channel.configure(channel::config::Config {
            timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })?;

        Ok(Self {
            channel,
            min_us,
            max_us,
            max_degrees,
        })
    }

    fn set_degrees(&mut self, degrees: u16) -> Result<()> {
        assert!(degrees <= self.max_degrees);
        let duty_pct = self.degrees_to_duty_pct(degrees);
        self.channel.set_duty(duty_pct)?;
        Ok(())
    }

    fn hold(&mut self) -> Result<()> {
        Ok(())
    }

    fn relax(&mut self) -> Result<()> {
        self.channel.set_duty(0)?;
        Ok(())
    }

    fn pulse_for_degrees(&self, degrees: u16) -> u32 {
        let pulse_span = self.max_us - self.min_us;
        self.min_us
            + (u32::from(degrees) * pulse_span + u32::from(self.max_degrees / 2))
                / u32::from(self.max_degrees)
    }

    fn degrees_to_duty_pct(&self, degrees: u16) -> u8 {
        let pulse_us = self.pulse_for_degrees(degrees);
        let duty_pct = ((pulse_us * 100) + (SERVO_PERIOD_US / 2)) / SERVO_PERIOD_US;
        assert!(duty_pct <= u8::MAX as u32);
        duty_pct as u8
    }
}

#[must_use]
const fn linear<const N: usize>(
    start_degrees: u16,
    end_degrees: u16,
    total_duration: embassy_time::Duration,
) -> [(u16, embassy_time::Duration); N] {
    assert!(N > 0, "at least one step required");
    let step_duration = Duration::from_micros(total_duration.as_micros() / (N as u64));
    let delta = end_degrees as i32 - start_degrees as i32;
    let denom = if N == 1 { 1 } else { (N - 1) as i32 };

    let mut result = [(0u16, Duration::from_micros(0)); N];
    let mut step_index = 0;
    while step_index < N {
        let degrees = if N == 1 {
            start_degrees
        } else {
            let step_delta = delta * (step_index as i32) / denom;
            (start_degrees as i32 + step_delta) as u16
        };
        result[step_index] = (degrees, step_duration);
        step_index += 1;
    }
    result
}

#[must_use]
const fn combine<const N1: usize, const N2: usize, const OUT_N: usize>(
    first: [(u16, embassy_time::Duration); N1],
    second: [(u16, embassy_time::Duration); N2],
) -> [(u16, embassy_time::Duration); OUT_N] {
    assert!(OUT_N == N1 + N2, "OUT_N must equal N1 + N2");

    let mut result = [(0u16, Duration::from_micros(0)); OUT_N];
    let mut first_index = 0;
    while first_index < N1 {
        result[first_index] = first[first_index];
        first_index += 1;
    }

    let mut second_index = 0;
    while second_index < N2 {
        result[N1 + second_index] = second[second_index];
        second_index += 1;
    }

    result
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
