//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides:
//! - [`ButtonEsp`] for direct debounced polling
//! - [`ButtonWatchEsp`] for background monitoring that avoids future-cancellation starvation

#[cfg(target_os = "none")]
mod button_watch;

#[cfg(target_os = "none")]
pub use button_watch::{ButtonWatchEsp, ButtonWatchStaticEsp};
pub use device_envoy_core::button::{Button as ButtonDevice, ButtonWatch as ButtonWatchDevice};
pub use device_envoy_core::button::{
    PressDuration, PressedTo, BUTTON_DEBOUNCE_DELAY, BUTTON_POLL_INTERVAL, LONG_PRESS_DURATION,
};

#[cfg(target_os = "none")]
use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

/// A button with debouncing and press duration detection.
#[cfg(target_os = "none")]
pub struct ButtonEsp<'d> {
    input: Input<'d>,
    pressed_to: PressedTo,
}

#[cfg(target_os = "none")]
impl<'d> ButtonEsp<'d> {
    /// Creates a new `ButtonEsp` instance from a pin.
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

    /// Wait for any button edge (press or release).
    pub async fn wait_for_any_edge(&mut self) {
        self.input.wait_for_any_edge().await;
    }

    /// Consumes the button and returns its internal components.
    ///
    /// This is useful for converting a `ButtonEsp` into a `ButtonWatchEsp`.
    #[doc(hidden)]
    #[must_use]
    pub fn into_parts(self) -> (Input<'d>, PressedTo) {
        (self.input, self.pressed_to)
    }
}

#[cfg(target_os = "none")]
impl device_envoy_core::button::Button for ButtonEsp<'_> {
    fn is_pressed(&self) -> bool {
        self.pressed_to.is_pressed(self.input.is_high())
    }

    // todo00000 test this. It used to poll.
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
