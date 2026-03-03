//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides:
//! - [`Button`] for direct debounced polling
//! - [`ButtonWatch`] for background monitoring that avoids future-cancellation starvation

#[cfg(target_os = "none")]
mod button_watch;

#[cfg(target_os = "none")]
pub use button_watch::{ButtonWatch, ButtonWatchStatic};
pub use device_envoy_core::button::{
    PressDuration, PressedTo, BUTTON_DEBOUNCE_DELAY, LONG_PRESS_DURATION,
};

#[cfg(target_os = "none")]
use embassy_futures::select::{select, Either};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};

#[cfg(target_os = "none")]
use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

/// A button with debouncing and press duration detection.
#[cfg(target_os = "none")]
pub struct Button<'d> {
    input: Input<'d>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
impl<'d> Button<'d> {
    /// Creates a new `Button` instance from a pin.
    ///
    /// The pin is configured based on the connection type:
    /// - [`PressedTo::Voltage`]: Uses internal pull-down (button to 3.3V)
    /// - [`PressedTo::Ground`]: Uses internal pull-up (button to GND)
    #[must_use]
    pub fn new(button_pin: impl InputPin + 'd, pressed_to: PressedTo) -> Self {
        let pull = match pressed_to {
            PressedTo::Voltage => Pull::Down,
            PressedTo::Ground => Pull::Up,
        };
        let input = Input::new(button_pin, InputConfig::default().with_pull(pull));
        Self { input, pressed_to }
    }

    /// Returns `true` when the button is currently pressed.
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        self.pressed_to.is_pressed(self.input.is_high())
    }

    #[inline]
    async fn wait_for_button_up(&mut self) {
        while self.is_pressed() {
            Timer::after(Duration::from_millis(1)).await;
        }
    }

    #[inline]
    async fn wait_for_button_down(&mut self) {
        while !self.is_pressed() {
            Timer::after(Duration::from_millis(1)).await;
        }
    }

    #[inline]
    async fn wait_for_stable_down(&mut self) {
        loop {
            self.wait_for_button_down().await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if self.is_pressed() {
                break;
            }
        }
    }

    #[inline]
    async fn wait_for_stable_up(&mut self) {
        loop {
            self.wait_for_button_up().await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if !self.is_pressed() {
                break;
            }
        }
    }

    /// Waits for the next press (button goes down, debounced). Does not wait for release.
    pub async fn wait_for_press(&mut self) {
        self.wait_for_stable_up().await;
        self.wait_for_stable_down().await;
    }

    /// Waits for the next press and returns whether it was short or long (debounced).
    ///
    /// Returns as soon as it can decide, so long presses are reported before release.
    pub async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.wait_for_stable_up().await;
        self.wait_for_stable_down().await;

        match select(self.wait_for_stable_up(), Timer::after(LONG_PRESS_DURATION)).await {
            Either::First(_) => PressDuration::Short,
            Either::Second(()) => PressDuration::Long,
        }
    }

    /// Waits until the button is released (debounced).
    pub async fn wait_for_release(&mut self) {
        self.wait_for_stable_up().await;
    }

    /// Wait for any button edge (press or release).
    pub async fn wait_for_any_edge(&mut self) {
        self.input.wait_for_any_edge().await;
    }

    /// Consumes the button and returns its internal components.
    ///
    /// This is useful for converting a `Button` into a `ButtonWatch`.
    #[doc(hidden)]
    #[must_use]
    pub fn into_parts(self) -> (Input<'d>, PressedTo) {
        (self.input, self.pressed_to)
    }
}
