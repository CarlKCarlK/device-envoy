//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`ButtonWatchEsp`] for usage.

#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(target_os = "none")]
use super::{ButtonEsp, PressDuration, PressedTo};
#[cfg(target_os = "none")]
use crate::{Error, Result};

/// Static resources for [`ButtonWatchEsp`].
#[cfg(target_os = "none")]
pub struct ButtonWatchStaticEsp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    is_pressed: AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchStaticEsp {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
        }
    }
}

/// A device abstraction for background button monitoring.
///
/// Use this type when you need press detection that is not disrupted by other
/// fast loops/tasks that repeatedly cancel futures.
#[cfg(target_os = "none")]
pub struct ButtonWatchEsp<'a> {
    signal: &'a Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'a Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'a AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchEsp<'_> {
    /// Create static resources for [`ButtonWatchEsp::new`] and [`ButtonWatchEsp::new_from_pin`].
    #[must_use]
    pub const fn new_static() -> ButtonWatchStaticEsp {
        ButtonWatchStaticEsp::new()
    }

    /// Create a background button monitor from an existing [`ButtonEsp`].
    ///
    /// This constructor spawns a dedicated task that continuously monitors the
    /// button and signals [`PressDuration`] events.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        button_watch_static: &'static ButtonWatchStaticEsp,
        button: ButtonEsp<'static>,
        spawner: Spawner,
    ) -> Result<Self> {
        let (input, pressed_to) = button.into_parts();
        let token = button_watch_task_from_input(
            input,
            pressed_to,
            &button_watch_static.signal,
            &button_watch_static.state_signal,
            &button_watch_static.is_pressed,
        );
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self {
            signal: &button_watch_static.signal,
            state_signal: &button_watch_static.state_signal,
            is_pressed: &button_watch_static.is_pressed,
        })
    }

    /// Create a background button monitor directly from a pin.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new_from_pin(
        button_watch_static: &'static ButtonWatchStaticEsp,
        button_pin: impl esp_hal::gpio::InputPin + 'static,
        pressed_to: PressedTo,
        spawner: Spawner,
    ) -> Result<Self> {
        let button = ButtonEsp::new(button_pin, pressed_to);
        Self::new(button_watch_static, button, spawner)
    }
}

#[cfg(target_os = "none")]
impl device_envoy_core::button::Button for ButtonWatchEsp<'_> {
    fn is_pressed(&self) -> bool {
        self.is_pressed.load(Ordering::Relaxed)
    }

    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.signal.wait().await
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        if self.is_pressed() == pressed {
            return;
        }

        loop {
            let state = self.state_signal.wait().await;
            if state == pressed {
                return;
            }
        }
    }
}

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn button_watch_task_from_input(
    mut input: esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'static AtomicBool,
) -> ! {
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    signal_press_durations(&mut input_button, signal, state_signal, is_pressed).await
}

#[cfg(target_os = "none")]
struct InputButton<'a> {
    input: &'a mut esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
impl device_envoy_core::button::Button for InputButton<'_> {
    fn is_pressed(&self) -> bool {
        self.pressed_to.is_pressed(self.input.is_high())
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        match (pressed, self.pressed_to) {
            (true, PressedTo::Voltage) | (false, PressedTo::Ground) => {
                self.input.wait_for_high().await;
            }
            (true, PressedTo::Ground) | (false, PressedTo::Voltage) => {
                self.input.wait_for_low().await;
            }
        }
    }
}

#[cfg(target_os = "none")]
async fn signal_press_durations<B: device_envoy_core::button::Button>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    is_pressed: &'static AtomicBool,
) -> ! {
    let initial_pressed = button.is_pressed();
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);

    loop {
        <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false).await;

        <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, true).await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);

        Timer::after(device_envoy_core::button::BUTTON_DEBOUNCE_DELAY).await;
        if !button.is_pressed() {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            continue;
        }

        let press_duration = embassy_futures::select::select(
            <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false),
            Timer::after(device_envoy_core::button::LONG_PRESS_DURATION),
        )
        .await;

        match press_duration {
            embassy_futures::select::Either::First(()) => {
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                signal.signal(PressDuration::Short);
            }
            embassy_futures::select::Either::Second(()) => {
                signal.signal(PressDuration::Long);
                <B as device_envoy_core::button::Button>::wait_until_pressed_state(button, false)
                    .await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
            }
        }
    }
}
