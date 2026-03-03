//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`ButtonWatch`] for usage.

#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

#[cfg(target_os = "none")]
use super::{Button, PressDuration, PressedTo};
#[cfg(target_os = "none")]
use crate::{Error, Result};

/// Static resources for [`ButtonWatch`].
#[cfg(target_os = "none")]
pub struct ButtonWatchStatic {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
}

#[cfg(target_os = "none")]
impl ButtonWatchStatic {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }
}

/// A device abstraction for background button monitoring.
///
/// Use this type when you need press detection that is not disrupted by other
/// fast loops/tasks that repeatedly cancel futures.
#[cfg(target_os = "none")]
pub struct ButtonWatch<'a> {
    signal: &'a Signal<CriticalSectionRawMutex, PressDuration>,
}

#[cfg(target_os = "none")]
impl ButtonWatch<'_> {
    /// Create static resources for [`ButtonWatch::new`] and [`ButtonWatch::new_from_pin`].
    #[must_use]
    pub const fn new_static() -> ButtonWatchStatic {
        ButtonWatchStatic::new()
    }

    /// Create a background button monitor from an existing [`Button`].
    ///
    /// This constructor spawns a dedicated task that continuously monitors the
    /// button and signals [`PressDuration`] events.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        button_watch_static: &'static ButtonWatchStatic,
        button: Button<'static>,
        spawner: Spawner,
    ) -> Result<Self> {
        let (input, pressed_to) = button.into_parts();
        let token = button_watch_task_from_input(input, pressed_to, &button_watch_static.signal);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self {
            signal: &button_watch_static.signal,
        })
    }

    /// Create a background button monitor directly from a pin.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new_from_pin(
        button_watch_static: &'static ButtonWatchStatic,
        button_pin: impl esp_hal::gpio::InputPin + 'static,
        pressed_to: PressedTo,
        spawner: Spawner,
    ) -> Result<Self> {
        let button = Button::new(button_pin, pressed_to);
        Self::new(button_watch_static, button, spawner)
    }

    /// Wait for the next short/long press event from the background monitor.
    pub async fn wait_for_press_duration(&self) -> PressDuration {
        self.signal.wait().await
    }
}

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn button_watch_task_from_input(
    mut input: esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
) -> ! {
    loop {
        // Wait for button-up (debounced).
        while pressed_to.is_pressed(input.is_high()) {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        }
        embassy_time::Timer::after(super::BUTTON_DEBOUNCE_DELAY).await;
        while pressed_to.is_pressed(input.is_high()) {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        }

        // Wait for button-down (debounced).
        while !pressed_to.is_pressed(input.is_high()) {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        }
        embassy_time::Timer::after(super::BUTTON_DEBOUNCE_DELAY).await;
        if !pressed_to.is_pressed(input.is_high()) {
            continue;
        }

        let press_duration = match embassy_futures::select::select(
            wait_for_release(&mut input, pressed_to),
            embassy_time::Timer::after(super::LONG_PRESS_DURATION),
        )
        .await
        {
            embassy_futures::select::Either::First(_) => PressDuration::Short,
            embassy_futures::select::Either::Second(()) => PressDuration::Long,
        };

        signal.signal(press_duration);
    }
}

#[cfg(target_os = "none")]
async fn wait_for_release(input: &mut esp_hal::gpio::Input<'static>, pressed_to: PressedTo) {
    loop {
        if !pressed_to.is_pressed(input.is_high()) {
            embassy_time::Timer::after(super::BUTTON_DEBOUNCE_DELAY).await;
            if !pressed_to.is_pressed(input.is_high()) {
                break;
            }
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
    }
}
