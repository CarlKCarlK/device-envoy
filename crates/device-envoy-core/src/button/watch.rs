//! Background button monitoring shared by the RP and ESP `button_watch!` macros.
//!
//! Each generated button owns a [`ButtonWatchStatic`] and spawns its own task, which samples
//! the pin through a chip-specific [`__ButtonMonitor`] and runs [`__run_button_watch`]. The
//! generated type reads the results through a [`ButtonWatch`] handle.

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;

use super::{__ButtonMonitor, BUTTON_DEBOUNCE_DELAY, Button, LONG_PRESS_DURATION, PressDuration};

/// Static resources shared by one button's background task and its [`ButtonWatch`] handle.
// Must be `pub` for macro expansion at downstream call sites: each generated button declares
// one as a `static`.
#[doc(hidden)]
pub struct ButtonWatchStatic {
    signal: Signal<CriticalSectionRawMutex, PressDuration>,
    state_signal: Signal<CriticalSectionRawMutex, bool>,
    initialized_signal: Signal<CriticalSectionRawMutex, ()>,
    is_pressed: AtomicBool,
    initialized: AtomicBool,
}

impl ButtonWatchStatic {
    /// Creates the static resources for one button.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            signal: Signal::new(),
            state_signal: Signal::new(),
            initialized_signal: Signal::new(),
            is_pressed: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        }
    }
}

impl Default for ButtonWatchStatic {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle that reads a button's state from its background task.
// Must be `pub` for macro expansion at downstream call sites: generated buttons wrap it.
#[doc(hidden)]
pub struct ButtonWatch {
    button_watch_static: &'static ButtonWatchStatic,
}

impl ButtonWatch {
    /// Creates a handle on a button's static resources.
    #[must_use]
    pub const fn new(button_watch_static: &'static ButtonWatchStatic) -> Self {
        Self {
            button_watch_static,
        }
    }

    /// Waits until the background task has sampled the button's first state.
    pub async fn wait_until_initialized(&self) {
        if self.button_watch_static.initialized.load(Ordering::Acquire) {
            return;
        }
        self.button_watch_static.initialized_signal.wait().await;
    }
}

impl __ButtonMonitor for ButtonWatch {
    fn is_pressed_raw(&self) -> bool {
        self.button_watch_static.is_pressed.load(Ordering::Relaxed)
    }

    async fn wait_until_pressed_state(&mut self, pressed: bool) {
        if self.button_watch_static.is_pressed.load(Ordering::Relaxed) == pressed {
            return;
        }

        loop {
            let state = self.button_watch_static.state_signal.wait().await;
            if state == pressed {
                return;
            }
        }
    }
}

impl Button for ButtonWatch {
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.button_watch_static.signal.wait().await
    }
}

/// Samples `button` forever, publishing its state and press durations to `button_watch_static`.
// Must be `pub` for macro expansion at downstream call sites: the RP and ESP task bodies call it.
#[doc(hidden)]
pub async fn __run_button_watch<B: __ButtonMonitor>(
    button: &mut B,
    button_watch_static: &'static ButtonWatchStatic,
) -> ! {
    let ButtonWatchStatic {
        signal,
        state_signal,
        initialized_signal,
        is_pressed,
        initialized,
    } = button_watch_static;

    let initial_pressed = button.is_pressed_raw();
    is_pressed.store(initial_pressed, Ordering::Relaxed);
    state_signal.signal(initial_pressed);
    initialized.store(true, Ordering::Release);
    initialized_signal.signal(());

    loop {
        button.wait_until_pressed_state(false).await;

        button.wait_until_pressed_state(true).await;
        is_pressed.store(true, Ordering::Relaxed);
        state_signal.signal(true);

        Timer::after(BUTTON_DEBOUNCE_DELAY).await;
        if !button.is_pressed_raw() {
            is_pressed.store(false, Ordering::Relaxed);
            state_signal.signal(false);
            continue;
        }

        let press_duration = select(
            button.wait_until_pressed_state(false),
            Timer::after(LONG_PRESS_DURATION),
        )
        .await;

        match press_duration {
            Either::First(()) => {
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
                signal.signal(PressDuration::Short);
            }
            Either::Second(()) => {
                signal.signal(PressDuration::Long);
                button.wait_until_pressed_state(false).await;
                is_pressed.store(false, Ordering::Relaxed);
                state_signal.signal(false);
            }
        }
    }
}
