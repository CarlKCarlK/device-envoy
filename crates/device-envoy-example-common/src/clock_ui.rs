use core::convert::{Infallible, TryFrom};

use device_envoy_core::{
    button::{Button, PressDuration},
    clock_sync::{ClockSync, ONE_DAY, ONE_MINUTE, ONE_SECOND, h12_m_s},
};
use embassy_futures::select::{Either, select};

const REAL_TIME_SPEED: f32 = 1.0;

/// Canonical clock UI configuration shared across example binaries.
pub struct ClockUiConfig {
    /// Speed used when switching from MM:SS back to HH:MM via short press.
    pub fast_mode_speed: f32,
    /// Minutes added on each short press while editing the timezone.
    pub edit_offset_step_minutes: i32,
}

impl Default for ClockUiConfig {
    fn default() -> Self {
        Self {
            fast_mode_speed: 720.0,
            edit_offset_step_minutes: 60,
        }
    }
}

/// Display operations needed by the shared clock UI state machine.
pub trait ClockUiDisplay {
    /// Render hours and minutes in normal mode.
    fn show_hours_minutes(&self, hours: u8, minutes: u8);

    /// Render minutes and seconds mode.
    fn show_minutes_seconds(&self, minutes: u8, seconds: u8);

    /// Render hours and minutes in timezone edit mode.
    fn show_hours_minutes_edit(&self, hours: u8, minutes: u8);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ClockUiState {
    HoursMinutes { speed: f32 },
    MinutesSeconds,
    EditOffset,
}

/// Run the canonical clock UI event loop.
///
/// This loop is based on the RP `clock_led4` behavior:
///
/// - Start in HH:MM mode at real-time speed.
/// - Short press in HH:MM enters MM:SS.
/// - Short press in MM:SS returns to HH:MM with fast mode.
/// - Any short press in HH:MM fast mode returns to HH:MM real-time mode.
/// - Long press in HH:MM or MM:SS enters timezone edit mode.
/// - In edit mode: short press increments offset; long press persists and exits.
pub async fn run_clock_ui<B, C, D, PersistOffset, E>(
    clock_sync: &C,
    button: &mut B,
    display: &D,
    mut persist_offset_minutes: PersistOffset,
    clock_ui_config: ClockUiConfig,
) -> Result<Infallible, E>
where
    B: Button,
    C: ClockSync,
    D: ClockUiDisplay,
    PersistOffset: FnMut(i32) -> Result<(), E>,
{
    assert!(
        clock_ui_config.edit_offset_step_minutes > 0,
        "edit_offset_step_minutes must be positive"
    );

    let mut clock_ui_state = ClockUiState::HoursMinutes {
        speed: REAL_TIME_SPEED,
    };
    loop {
        clock_ui_state = match clock_ui_state {
            ClockUiState::HoursMinutes { speed } => {
                run_hours_minutes_state(speed, clock_sync, button, display).await
            }
            ClockUiState::MinutesSeconds => {
                run_minutes_seconds_state(
                    clock_sync,
                    button,
                    display,
                    clock_ui_config.fast_mode_speed,
                )
                .await
            }
            ClockUiState::EditOffset => {
                run_edit_offset_state(
                    clock_sync,
                    button,
                    display,
                    &mut persist_offset_minutes,
                    clock_ui_config.edit_offset_step_minutes,
                )
                .await?
            }
        };
    }
}

async fn run_hours_minutes_state<B, C, D>(
    speed: f32,
    clock_sync: &C,
    button: &mut B,
    display: &D,
) -> ClockUiState
where
    B: Button,
    C: ClockSync,
    D: ClockUiDisplay,
{
    clock_sync.set_speed(speed);
    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
    display.show_hours_minutes(hours, minutes);
    clock_sync.set_tick_interval(Some(ONE_MINUTE));

    loop {
        match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
            Either::First(press_duration) => match (press_duration, speed.to_bits()) {
                (PressDuration::Short, bits) if bits == REAL_TIME_SPEED.to_bits() => {
                    return ClockUiState::MinutesSeconds;
                }
                (PressDuration::Short, _) => {
                    return ClockUiState::HoursMinutes {
                        speed: REAL_TIME_SPEED,
                    };
                }
                (PressDuration::Long, _) => {
                    return ClockUiState::EditOffset;
                }
            },
            Either::Second(clock_sync_tick) => {
                let (hours, minutes, _) = h12_m_s(&clock_sync_tick.local_time);
                display.show_hours_minutes(hours, minutes);
            }
        }
    }
}

async fn run_minutes_seconds_state<B, C, D>(
    clock_sync: &C,
    button: &mut B,
    display: &D,
    fast_mode_speed: f32,
) -> ClockUiState
where
    B: Button,
    C: ClockSync,
    D: ClockUiDisplay,
{
    clock_sync.set_speed(REAL_TIME_SPEED);
    let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
    display.show_minutes_seconds(minutes, seconds);
    clock_sync.set_tick_interval(Some(ONE_SECOND));

    loop {
        match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
            Either::First(PressDuration::Short) => {
                return ClockUiState::HoursMinutes {
                    speed: fast_mode_speed,
                };
            }
            Either::First(PressDuration::Long) => {
                return ClockUiState::EditOffset;
            }
            Either::Second(clock_sync_tick) => {
                let (_, minutes, seconds) = h12_m_s(&clock_sync_tick.local_time);
                display.show_minutes_seconds(minutes, seconds);
            }
        }
    }
}

async fn run_edit_offset_state<B, C, D, PersistOffset, E>(
    clock_sync: &C,
    button: &mut B,
    display: &D,
    persist_offset_minutes: &mut PersistOffset,
    edit_offset_step_minutes: i32,
) -> Result<ClockUiState, E>
where
    B: Button,
    C: ClockSync,
    D: ClockUiDisplay,
    PersistOffset: FnMut(i32) -> Result<(), E>,
{
    clock_sync.set_speed(REAL_TIME_SPEED);
    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
    display.show_hours_minutes_edit(hours, minutes);

    let mut offset_minutes = clock_sync.offset_minutes();
    clock_sync.set_tick_interval(None);

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                offset_minutes = increment_offset_minutes(offset_minutes, edit_offset_step_minutes);
                clock_sync.set_offset_minutes(offset_minutes);
                let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
                display.show_hours_minutes_edit(hours, minutes);
            }
            PressDuration::Long => {
                persist_offset_minutes(offset_minutes)?;
                return Ok(ClockUiState::HoursMinutes {
                    speed: REAL_TIME_SPEED,
                });
            }
        }
    }
}

fn increment_offset_minutes(offset_minutes: i32, edit_offset_step_minutes: i32) -> i32 {
    let one_day_minutes =
        i32::try_from(ONE_DAY.as_secs() / 60).expect("ONE_DAY minutes should fit i32");
    let mut next_offset_minutes = offset_minutes + edit_offset_step_minutes;
    while next_offset_minutes >= one_day_minutes {
        next_offset_minutes -= one_day_minutes;
    }
    next_offset_minutes
}
