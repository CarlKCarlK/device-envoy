//! Wi-Fi enabled 4-digit clock that provisions credentials through `WifiAuto`.
//!
//! This example demonstrates how to pair the shared captive-portal workflow with the
//! 4-digit LED clock state machine. The `WifiAuto` helper owns Wi-Fi onboarding while the
//! clock display reflects progress and, once connected, continues handling user input.
//!
//! Hardware defaults:
//! - force-portal button on GPIO6 (wired to GND)
//! - LED4 cell pins: GPIO14, GPIO13, GPIO12, GPIO11 (active-low)
//! - LED4 segment pins: GPIO10, GPIO9, GPIO46 (S3) / GPIO4 (C6), GPIO3, GPIO8, GPIO18, GPIO17, GPIO16 (active-high)

#![no_std]
#![no_main]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use log::info;

use device_envoy_esp::{
    button::{ButtonWatchEsp, ButtonWatchStaticEsp, PressDuration, PressedTo},
    clock_sync::{h12_m_s, ClockSync, ClockSyncStatic, ONE_DAY, ONE_MINUTE, ONE_SECOND},
    flash_block::FlashBlockEsp,
    init_and_start,
    led4::{circular_outline_animation, BlinkState, Led4, Led4Static, OutputArray},
    wifi_auto::{
        fields::{TimezoneField, TimezoneFieldStatic},
        WifiAuto, WifiAutoEvent,
    },
    Error, Result,
};

use device_envoy_esp::button::ButtonWatch as _;

esp_bootloader_esp_idf::esp_app_desc!();

const CAPTIVE_PORTAL_SSID: &str = "EnvoyClock4";
const FAST_MODE_SPEED: f32 = 720.0;

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

    info!("Starting Wi-Fi 4-digit clock (WifiAuto)");

    let [wifi_auto_flash_block, timezone_flash_block] = FlashBlockEsp::new_array::<2>(p.FLASH)?;

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

    let cell_pins = OutputArray::new([
        Output::new(p.GPIO14, Level::High, OutputConfig::default()),
        Output::new(p.GPIO13, Level::High, OutputConfig::default()),
        Output::new(p.GPIO12, Level::High, OutputConfig::default()),
        Output::new(p.GPIO11, Level::High, OutputConfig::default()),
    ]);

    let segment_pins = OutputArray::new([
        Output::new(p.GPIO10, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO9, Level::Low, OutputConfig::default()),
        #[cfg(target_arch = "xtensa")]
        Output::new(p.GPIO46, Level::Low, OutputConfig::default()),
        #[cfg(not(target_arch = "xtensa"))]
        Output::new(p.GPIO4, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO3, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO8, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO18, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO17, Level::Low, OutputConfig::default()),
        Output::new(p.GPIO16, Level::Low, OutputConfig::default()),
    ]);

    static LED4_STATIC: Led4Static = Led4::new_static();
    let led4 = Led4::new(&LED4_STATIC, cell_pins, segment_pins, spawner)?;

    let led4_ref = &led4;
    let (stack, button) = wifi_auto
        .connect(|wifi_auto_event| async move {
            match wifi_auto_event {
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

    static BUTTON_WATCH_STATIC: ButtonWatchStaticEsp = ButtonWatchEsp::new_static();
    let button_watch = ButtonWatchEsp::new(&BUTTON_WATCH_STATIC, button, spawner)?;

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

    led4.animate_text(circular_outline_animation(true));
    info!("Waiting for first NTP sync");
    let _clock_sync_tick = clock_sync.wait_for_tick().await;
    info!("First NTP sync complete");

    let mut state = State::HoursMinutes { speed: 1.0 };
    loop {
        state = match state {
            State::HoursMinutes { speed } => {
                state
                    .execute_hours_minutes(speed, &clock_sync, &button_watch, &led4)
                    .await?
            }
            State::MinutesSeconds => {
                state
                    .execute_minutes_seconds(&clock_sync, &button_watch, &led4)
                    .await?
            }
            State::EditOffset => {
                state
                    .execute_edit_offset(&clock_sync, &button_watch, timezone_field, &led4)
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
        button_watch: &ButtonWatchEsp<'_>,
        led4: &Led4<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(speed).await;
        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_hours(hours),
                Self::ones_digit(hours),
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
            ],
            BlinkState::Solid,
        );
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
                    led4.write_text(
                        [
                            Self::tens_hours(hours),
                            Self::ones_digit(hours),
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                        ],
                        BlinkState::Solid,
                    );
                }
            }
        }
    }

    async fn execute_minutes_seconds(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatchEsp<'_>,
        led4: &Led4<'_>,
    ) -> Result<Self> {
        clock_sync.set_speed(1.0).await;
        let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
                Self::tens_digit(seconds),
                Self::ones_digit(seconds),
            ],
            BlinkState::Solid,
        );
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
                    led4.write_text(
                        [
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                            Self::tens_digit(seconds),
                            Self::ones_digit(seconds),
                        ],
                        BlinkState::Solid,
                    );
                }
            }
        }
    }

    async fn execute_edit_offset(
        self,
        clock_sync: &ClockSync,
        button_watch: &ButtonWatchEsp<'_>,
        timezone_field: &TimezoneField,
        led4: &Led4<'_>,
    ) -> Result<Self> {
        info!("Entering edit offset mode");

        let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
        led4.write_text(
            [
                Self::tens_hours(hours),
                Self::ones_digit(hours),
                Self::tens_digit(minutes),
                Self::ones_digit(minutes),
            ],
            BlinkState::BlinkingAndOn,
        );

        let mut offset_minutes = clock_sync.offset_minutes();
        info!("Current offset: {} minutes", offset_minutes);

        clock_sync.set_tick_interval(None).await;
        clock_sync.set_speed(1.0).await;

        loop {
            match button_watch.wait_for_press_duration().await {
                PressDuration::Short => {
                    offset_minutes += 60;
                    const ONE_DAY_MINUTES: i32 = ONE_DAY.as_secs() as i32 / 60;
                    if offset_minutes >= ONE_DAY_MINUTES {
                        offset_minutes -= ONE_DAY_MINUTES;
                    }
                    clock_sync.set_offset_minutes(offset_minutes).await;
                    info!("New offset: {} minutes", offset_minutes);

                    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
                    led4.write_text(
                        [
                            Self::tens_hours(hours),
                            Self::ones_digit(hours),
                            Self::tens_digit(minutes),
                            Self::ones_digit(minutes),
                        ],
                        BlinkState::BlinkingAndOn,
                    );
                }
                PressDuration::Long => {
                    timezone_field.set_offset_minutes(offset_minutes)?;
                    info!("Offset saved to flash: {} minutes", offset_minutes);
                    return Ok(Self::HoursMinutes { speed: 1.0 });
                }
            }
        }
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
        if value >= 10 {
            '1'
        } else {
            ' '
        }
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
}
