use core::convert::{Infallible, TryFrom};

use device_envoy_core::{
    button::{Button, PressDuration},
    clock_sync::{ClockSync, ONE_DAY, ONE_MINUTE, ONE_SECOND, h12_m_s},
    flash_block::FlashBlock,
};
use embassy_futures::select::{Either, select};

const REAL_TIME_SPEED: f32 = 1.0;
const FAST_MODE_SPEED: f32 = 720.0;
const EDIT_OFFSET_STEP_MINUTES: i32 = 60;

/// Events emitted by the shared clock UI state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockUiEvent {
    /// Render hours and minutes in normal mode.
    RenderHoursMinutes { hours: u8, minutes: u8 },
    /// Render minutes and seconds mode.
    RenderMinutesSeconds { minutes: u8, seconds: u8 },
    /// Render hours and minutes in timezone edit mode.
    RenderHoursMinutesEdit { hours: u8, minutes: u8 },
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
/// - In edit mode: short press increments offset; long press requests persist then exits.
pub async fn run_clock_ui<B, C, F, OnEvent, OnEventFuture, E>(
    clock_sync: &C,
    button: &mut B,
    timezone_flash_block: &mut F,
    mut on_event: OnEvent,
) -> Result<Infallible, E>
where
    B: Button,
    C: ClockSync,
    F: FlashBlock<Error = E>,
    OnEvent: FnMut(ClockUiEvent) -> OnEventFuture,
    OnEventFuture: core::future::Future<Output = Result<(), E>>,
{
    let mut clock_ui_state = ClockUiState::HoursMinutes {
        speed: REAL_TIME_SPEED,
    };
    loop {
        clock_ui_state = match clock_ui_state {
            ClockUiState::HoursMinutes { speed } => {
                run_hours_minutes_state(speed, clock_sync, button, &mut on_event).await?
            }
            ClockUiState::MinutesSeconds => {
                run_minutes_seconds_state(clock_sync, button, &mut on_event).await?
            }
            ClockUiState::EditOffset => {
                run_edit_offset_state(clock_sync, button, timezone_flash_block, &mut on_event)
                    .await?
            }
        };
    }
}

async fn run_hours_minutes_state<B, C, OnEvent, OnEventFuture, E>(
    speed: f32,
    clock_sync: &C,
    button: &mut B,
    on_event: &mut OnEvent,
) -> Result<ClockUiState, E>
where
    B: Button,
    C: ClockSync,
    OnEvent: FnMut(ClockUiEvent) -> OnEventFuture,
    OnEventFuture: core::future::Future<Output = Result<(), E>>,
{
    clock_sync.set_speed(speed);
    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
    on_event(ClockUiEvent::RenderHoursMinutes { hours, minutes }).await?;
    clock_sync.set_tick_interval(Some(ONE_MINUTE));

    loop {
        match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
            Either::First(press_duration) => match (press_duration, speed.to_bits()) {
                (PressDuration::Short, bits) if bits == REAL_TIME_SPEED.to_bits() => {
                    return Ok(ClockUiState::MinutesSeconds);
                }
                (PressDuration::Short, _) => {
                    return Ok(ClockUiState::HoursMinutes {
                        speed: REAL_TIME_SPEED,
                    });
                }
                (PressDuration::Long, _) => {
                    return Ok(ClockUiState::EditOffset);
                }
            },
            Either::Second(clock_sync_tick) => {
                let (hours, minutes, _) = h12_m_s(&clock_sync_tick.local_time);
                on_event(ClockUiEvent::RenderHoursMinutes { hours, minutes }).await?;
            }
        }
    }
}

async fn run_minutes_seconds_state<B, C, OnEvent, OnEventFuture, E>(
    clock_sync: &C,
    button: &mut B,
    on_event: &mut OnEvent,
) -> Result<ClockUiState, E>
where
    B: Button,
    C: ClockSync,
    OnEvent: FnMut(ClockUiEvent) -> OnEventFuture,
    OnEventFuture: core::future::Future<Output = Result<(), E>>,
{
    clock_sync.set_speed(REAL_TIME_SPEED);
    let (_, minutes, seconds) = h12_m_s(&clock_sync.now_local());
    on_event(ClockUiEvent::RenderMinutesSeconds { minutes, seconds }).await?;
    clock_sync.set_tick_interval(Some(ONE_SECOND));

    loop {
        match select(button.wait_for_press_duration(), clock_sync.wait_for_tick()).await {
            Either::First(PressDuration::Short) => {
                return Ok(ClockUiState::HoursMinutes {
                    speed: FAST_MODE_SPEED,
                });
            }
            Either::First(PressDuration::Long) => {
                return Ok(ClockUiState::EditOffset);
            }
            Either::Second(clock_sync_tick) => {
                let (_, minutes, seconds) = h12_m_s(&clock_sync_tick.local_time);
                on_event(ClockUiEvent::RenderMinutesSeconds { minutes, seconds }).await?;
            }
        }
    }
}

async fn run_edit_offset_state<B, C, F, OnEvent, OnEventFuture, E>(
    clock_sync: &C,
    button: &mut B,
    timezone_flash_block: &mut F,
    on_event: &mut OnEvent,
) -> Result<ClockUiState, E>
where
    B: Button,
    C: ClockSync,
    F: FlashBlock<Error = E>,
    OnEvent: FnMut(ClockUiEvent) -> OnEventFuture,
    OnEventFuture: core::future::Future<Output = Result<(), E>>,
{
    clock_sync.set_speed(REAL_TIME_SPEED);
    let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
    on_event(ClockUiEvent::RenderHoursMinutesEdit { hours, minutes }).await?;

    let mut offset_minutes = clock_sync.offset_minutes();
    clock_sync.set_tick_interval(None);

    loop {
        match button.wait_for_press_duration().await {
            PressDuration::Short => {
                offset_minutes = increment_offset_minutes(offset_minutes, EDIT_OFFSET_STEP_MINUTES);
                clock_sync.set_offset_minutes(offset_minutes);
                let (hours, minutes, _) = h12_m_s(&clock_sync.now_local());
                on_event(ClockUiEvent::RenderHoursMinutesEdit { hours, minutes }).await?;
            }
            PressDuration::Long => {
                timezone_flash_block.save(&offset_minutes)?;
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
