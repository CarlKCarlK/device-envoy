//! Platform-independent button types and constants.
//!
//! See the platform-specific crate (for example `device_envoy_rp::button` or
//! `device_envoy_esp::button`) for the primary documentation and examples.

use embassy_futures::select::{Either, select};
use embassy_time::Duration;
use embassy_time::Timer;

// ============================================================================
// Constants
// ============================================================================

/// Debounce delay for the button.
pub const BUTTON_DEBOUNCE_DELAY: Duration = Duration::from_millis(10);

/// Duration representing a long button press.
pub const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

/// Polling interval used by default button wait helpers.
pub const BUTTON_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Platform-agnostic button contract.
///
/// Platform crates implement this for concrete button types and inherit the default
/// debouncing and press-duration behavior from shared core logic.
#[allow(async_fn_in_trait)]
pub trait Button {
    /// Returns whether the button is currently pressed.
    fn is_pressed(&self) -> bool;

    /// Wait until the sampled pressed state matches `pressed`.
    ///
    /// Implementations may use edge interrupts, polling, or any platform-specific mechanism.
    async fn wait_until_pressed_state(&mut self, pressed: bool);

    #[inline]
    async fn wait_for_button_up(&mut self) -> &mut Self {
        self.wait_until_pressed_state(false).await;
        self
    }

    #[inline]
    async fn wait_for_button_down(&mut self) -> &mut Self {
        self.wait_until_pressed_state(true).await;
        self
    }

    #[inline]
    async fn wait_for_stable_down(&mut self) -> &mut Self {
        loop {
            self.wait_for_button_down().await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if self.is_pressed() {
                break;
            }
            // otherwise it was bounce; keep waiting
        }
        self
    }

    #[inline]
    async fn wait_for_stable_up(&mut self) -> &mut Self {
        loop {
            self.wait_for_button_up().await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if !self.is_pressed() {
                break;
            }
        }
        self
    }

    /// Waits for the next press (button goes down, debounced). Does not wait for release.
    async fn wait_for_press(&mut self) {
        self.wait_for_stable_up().await;
        self.wait_for_stable_down().await;
    }

    /// Waits for the next press and returns whether it was short or long (debounced).
    ///
    /// Returns as soon as it can decide, so long presses are reported before release.
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.wait_for_stable_up().await;
        self.wait_for_stable_down().await;

        match select(self.wait_for_stable_up(), Timer::after(LONG_PRESS_DURATION)).await {
            Either::First(_) => PressDuration::Short,
            Either::Second(()) => PressDuration::Long,
        }
    }

    /// Waits until the button is released (debounced).
    async fn wait_for_release(&mut self) {
        self.wait_for_stable_up().await;
    }
}

/// Platform-agnostic background button monitor contract.
#[allow(async_fn_in_trait)]
pub trait ButtonWatch {
    /// Wait for the next short/long press event from the background monitor.
    async fn wait_for_press_duration(&self) -> PressDuration;
}

// ============================================================================
// PressedTo - How the button is wired
// ============================================================================

/// Describes if the button connects to voltage or ground when pressed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

    /// Evaluates whether the button is pressed for a sampled logic level.
    #[must_use]
    pub const fn is_pressed(self, level_is_high: bool) -> bool {
        if self.pressed_is_high() {
            level_is_high
        } else {
            !level_is_high
        }
    }
}

// ============================================================================
// PressDuration - Button press type
// ============================================================================

/// Duration of a button press (short or long).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PressDuration {
    /// Button was held for less than [`LONG_PRESS_DURATION`] (500ms).
    Short,
    /// Button was held for at least [`LONG_PRESS_DURATION`] (500ms).
    Long,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
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
