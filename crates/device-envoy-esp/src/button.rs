//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides a reusable button device for ESP32 projects.

#[cfg(target_os = "none")]
use embassy_futures::select::{select, Either};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};

#[cfg(target_os = "none")]
use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

/// Debounce delay for the button.
#[cfg(target_os = "none")]
pub(crate) const BUTTON_DEBOUNCE_DELAY: Duration = Duration::from_millis(10);

/// Duration representing a long button press.
#[cfg(target_os = "none")]
pub(crate) const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

/// Describes if the button connects to voltage or ground when pressed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressedTo {
    /// Button connects pin to voltage (3.3V) when pressed.
    /// Uses internal pull-down resistor. Pin reads HIGH when pressed.
    Voltage,
    /// Button connects pin to ground (GND) when pressed.
    /// Uses internal pull-up resistor. Pin reads LOW when pressed.
    Ground,
}

impl PressedTo {
    /// Returns `true` when a high input level means "pressed".
    #[must_use]
    pub const fn pressed_is_high(self) -> bool {
        matches!(self, Self::Voltage)
    }

    /// Evaluate whether the button is pressed for a sampled logic level.
    #[must_use]
    pub const fn is_pressed(self, level_is_high: bool) -> bool {
        if self.pressed_is_high() {
            level_is_high
        } else {
            !level_is_high
        }
    }
}

/// Duration of a button press (short or long).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressDuration {
    /// Button was held for less than 500 ms.
    Short,
    /// Button was held for at least 500 ms.
    Long,
}

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
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::PressedTo;

    #[test]
    fn pressed_to_ground_maps_low_to_pressed() {
        assert!(PressedTo::Ground.is_pressed(false));
        assert!(!PressedTo::Ground.is_pressed(true));
    }

    #[test]
    fn pressed_to_voltage_maps_high_to_pressed() {
        assert!(!PressedTo::Voltage.is_pressed(false));
        assert!(PressedTo::Voltage.is_pressed(true));
    }
}
