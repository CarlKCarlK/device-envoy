//! A device abstraction for background button monitoring with a spawned task.
//!
//! See [`button_watch!`](crate::button_watch!) for usage.

#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "none")]
use device_envoy_core::button::{__ButtonMonitor, Button};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(target_os = "none")]
use super::{PressDuration, PressedTo};
#[cfg(target_os = "none")]
use crate::{Error, Result};

/// Static resources for [`ButtonWatchEsp`].
#[cfg(target_os = "none")]
// Public only because `button_watch!` macro expansions reference this type in
// downstream crates.
#[doc(hidden)]
pub struct ButtonWatchStaticEsp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: Signal<CriticalSectionRawMutex, ()>,
    is_pressed: AtomicBool,
    initialized: AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchStaticEsp {
    const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            state_changed_signal: Signal::new(),
            initialized_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }
}

/// A device abstraction for background button monitoring.
///
/// Use this type when you need press detection that is not disrupted by other
/// fast loops/tasks that repeatedly cancel futures.
#[cfg(target_os = "none")]
// Public only because `button_watch!` macro expansions reference this type in
// downstream crates.
#[doc(hidden)]
pub struct ButtonWatchEsp<'a> {
    signal: &'a Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'a Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'a Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'a Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'a AtomicBool,
    initialized: &'a AtomicBool,
}

#[cfg(target_os = "none")]
impl ButtonWatchEsp<'_> {
    /// Create static resources for [`ButtonWatchEsp::new_from_pin`].
    #[must_use]
    pub const fn new_static() -> ButtonWatchStaticEsp {
        ButtonWatchStaticEsp::new()
    }

    /// Create a background button monitor directly from a pin.
    #[must_use = "Must be used to manage the spawned task"]
    pub async fn new_from_pin(
        button_watch_static: &'static ButtonWatchStaticEsp,
        button_pin: impl esp_hal::gpio::InputPin + 'static,
        pressed_to: PressedTo,
        spawner: Spawner,
    ) -> Result<Self> {
        let pull = match pressed_to {
            PressedTo::Voltage => esp_hal::gpio::Pull::Down,
            PressedTo::Ground => esp_hal::gpio::Pull::Up,
        };
        let input = esp_hal::gpio::Input::new(
            button_pin,
            esp_hal::gpio::InputConfig::default().with_pull(pull),
        );
        let token = button_watch_task_from_pin(
            input,
            pressed_to,
            &button_watch_static.signal,
            &button_watch_static.state_signal,
            &button_watch_static.state_changed_signal,
            &button_watch_static.initialized_signal,
            &button_watch_static.is_pressed,
            &button_watch_static.initialized,
        );
        spawner.spawn(token.map_err(Error::TaskSpawn)?);
        let button_watch = Self {
            signal: &button_watch_static.signal,
            state_signal: &button_watch_static.state_signal,
            state_changed_signal: &button_watch_static.state_changed_signal,
            initialized_signal: &button_watch_static.initialized_signal,
            is_pressed: &button_watch_static.is_pressed,
            initialized: &button_watch_static.initialized,
        };
        button_watch.wait_until_initialized().await;
        Ok(button_watch)
    }

    /// Wait until the first sampled state has been captured by the background task.
    pub async fn wait_until_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }
        self.initialized_signal.wait().await;
    }

    /// Wait for any button state transition (press or release).
    pub async fn wait_for_state_change(&self) {
        self.state_changed_signal.wait().await;
    }
}

#[cfg(target_os = "none")]
impl __ButtonMonitor for ButtonWatchEsp<'_> {
    fn is_pressed_raw(&self) -> bool {
        self.is_pressed.load(Ordering::Relaxed)
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        if self.is_pressed.load(Ordering::Relaxed) == pressed {
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

#[cfg(target_os = "none")]
impl Button for ButtonWatchEsp<'_> {
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.signal.wait().await
    }
}

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn button_watch_task_from_pin(
    mut input: esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
    let mut input_button = InputButton {
        input: &mut input,
        pressed_to,
    };
    signal_press_durations(
        &mut input_button,
        signal,
        state_signal,
        state_changed_signal,
        initialized_signal,
        is_pressed,
        initialized,
    )
    .await
}

#[cfg(target_os = "none")]
struct InputButton<'a> {
    input: &'a mut esp_hal::gpio::Input<'static>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
impl __ButtonMonitor for InputButton<'_> {
    fn is_pressed_raw(&self) -> bool {
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
async fn signal_press_durations<B: __ButtonMonitor>(
    button: &mut B,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
    let initial_pressed = B::is_pressed_raw(button);
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);
    initialized.store(true, Ordering::Release);
    initialized_signal.signal(());

    loop {
        B::wait_until_pressed_state(button, false).await;

        B::wait_until_pressed_state(button, true).await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);
        state_changed_signal.signal(());

        Timer::after(device_envoy_core::button::BUTTON_DEBOUNCE_DELAY).await;
        if !B::is_pressed_raw(button) {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            state_changed_signal.signal(());
            continue;
        }

        let press_duration = embassy_futures::select::select(
            B::wait_until_pressed_state(button, false),
            Timer::after(device_envoy_core::button::LONG_PRESS_DURATION),
        )
        .await;

        match press_duration {
            embassy_futures::select::Either::First(()) => {
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                state_changed_signal.signal(());
                signal.signal(PressDuration::Short);
            }
            embassy_futures::select::Either::Second(()) => {
                signal.signal(PressDuration::Long);
                B::wait_until_pressed_state(button, false).await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                state_changed_signal.signal(());
            }
        }
    }
}
