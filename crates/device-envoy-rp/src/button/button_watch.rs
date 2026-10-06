//! Background button monitoring with a spawned task.
//!
//! See the [`button_watch!`](crate::button_watch!) macro for usage and
//! [`ButtonWatchGenerated`](super::button_watch_generated::ButtonWatchGenerated) for a sample of a generated type.

use core::sync::atomic::{AtomicBool, Ordering};
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use super::{PressDuration, PressedTo};

// ============================================================================
// ButtonWatchStaticRp - Static resources for button monitoring
// ============================================================================

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub struct ButtonWatchStaticRp {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: Signal<CriticalSectionRawMutex, ()>,
    is_pressed: AtomicBool,
    initialized: AtomicBool,
}

impl ButtonWatchStaticRp {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            state_changed_signal: Signal::new(),
            initialized_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub const fn signal(&self) -> &Signal<CriticalSectionRawMutex, PressDuration> {
        &self.signal
    }

    #[must_use]
    pub const fn state_signal(&self) -> &Signal<CriticalSectionRawMutex, bool> {
        &self.state_signal
    }

    #[must_use]
    pub const fn state_changed_signal(&self) -> &Signal<CriticalSectionRawMutex, ()> {
        &self.state_changed_signal
    }

    #[must_use]
    pub const fn initialized_signal(&self) -> &Signal<CriticalSectionRawMutex, ()> {
        &self.initialized_signal
    }

    #[must_use]
    pub const fn is_pressed(&self) -> &AtomicBool {
        &self.is_pressed
    }

    #[must_use]
    pub const fn initialized(&self) -> &AtomicBool {
        &self.initialized
    }
}

// ============================================================================
// ButtonWatchRp - Handle for background button monitoring
// ============================================================================

// Must be public for macro expansion in downstream crates, but not user-facing API.
// Users interact with the macro-generated structs (e.g., ButtonWatchGenerated), not this type directly.
#[doc(hidden)]
pub struct ButtonWatchRp {
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
}

impl ButtonWatchRp {
    #[must_use]
    pub fn new(button_watch_static: &'static ButtonWatchStaticRp) -> Self {
        Self {
            signal: button_watch_static.signal(),
            state_signal: button_watch_static.state_signal(),
            state_changed_signal: button_watch_static.state_changed_signal(),
            initialized_signal: button_watch_static.initialized_signal(),
            is_pressed: button_watch_static.is_pressed(),
            initialized: button_watch_static.initialized(),
        }
    }

    pub async fn wait_until_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }
        self.initialized_signal.wait().await;
    }

    pub async fn wait_for_state_change(&self) {
        self.state_changed_signal.wait().await;
    }
}

impl device_envoy_core::button::__ButtonMonitor for ButtonWatchRp {
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

impl device_envoy_core::button::Button for ButtonWatchRp {
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.signal.wait().await
    }
}

// ============================================================================
// Background task implementation
// ============================================================================

/// Background task that monitors button state and fires events.
///
/// Never call directly - spawned automatically by the [`button_watch!`](crate::button_watch!) macro.
#[doc(hidden)]
pub async fn button_watch_task<P: Pin>(
    pin: Peri<'static, P>,
    pressed_to: PressedTo,
    signal: &'static Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: &'static Signal<CriticalSectionRawMutex, bool>,
    state_changed_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    initialized_signal: &'static Signal<CriticalSectionRawMutex, ()>,
    is_pressed: &'static AtomicBool,
    initialized: &'static AtomicBool,
) -> ! {
    let pull = match pressed_to {
        PressedTo::Voltage => Pull::Down,
        PressedTo::Ground => Pull::Up,
    };
    let mut input = Input::new(pin, pull);
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

struct InputButton<'a> {
    input: &'a mut Input<'static>,
    pressed_to: PressedTo,
}

impl device_envoy_core::button::__ButtonMonitor for InputButton<'_> {
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

async fn signal_press_durations<B: device_envoy_core::button::__ButtonMonitor>(
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

// ============================================================================
// button_watch! macro
// ============================================================================
