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

/// Internal primitive methods used to build the public [`Button`] API.
///
/// Platform crates implement this for concrete button types.
#[allow(async_fn_in_trait)]
#[doc(hidden)]
pub trait __ButtonMonitor {
    /// Returns whether the button is currently pressed.
    fn is_pressed_raw(&self) -> bool;

    /// Wait until the sampled pressed state matches `pressed`.
    ///
    /// Implementations may use edge interrupts, polling, or any platform-specific mechanism.
    async fn wait_until_pressed_state(&mut self, pressed: bool);
}

/// Platform-agnostic button contract.
///
/// Platform crates inherit the default debouncing and press-duration behavior from shared
/// core logic by implementing [`__ButtonMonitor`].
///
/// # Hardware Requirements
///
/// The button can be wired in two ways:
///
/// - [`PressedTo::Voltage`]: Button connects pin to voltage when pressed (active-high)
/// - [`PressedTo::Ground`]: Button connects pin to ground when pressed (active-low)
///
/// # Usage
///
/// Use [`Button::wait_for_press`] when you only need a debounced
/// press event. It returns on the down edge and does not wait for release.
///
/// Use [`Button::wait_for_press_duration`] when you need to
/// distinguish short vs. long presses. It returns as soon as it can decide, so long
/// presses are reported before the button is released.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::button::{Button, PressDuration};
///
/// async fn log_button_presses(button: &mut impl Button) -> ! {
///     // Wait for a press without measuring duration.
///     button.wait_for_press().await;
///
///     // Measure press durations in a loop.
///     loop {
///         match button.wait_for_press_duration().await {
///             PressDuration::Short => {
///                 // Handle short press.
///             }
///             PressDuration::Long => {
///                 // Handle long press (fires before button is released).
///             }
///         }
///     }
/// }
///
/// # struct ButtonMock;
/// # impl device_envoy_core::button::__ButtonMonitor for ButtonMock {
/// #     fn is_pressed_raw(&self) -> bool { false }
/// #     async fn wait_until_pressed_state(&mut self, _pressed: bool) {}
/// # }
/// # impl Button for ButtonMock {}
/// # fn main() {
/// #     let mut button = ButtonMock;
/// #     let _future = log_button_presses(&mut button);
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait Button: __ButtonMonitor {
    /// Returns whether the button is currently pressed.
    fn is_pressed(&self) -> bool {
        <Self as __ButtonMonitor>::is_pressed_raw(self)
    }

    /// Waits for the next press (button goes down, debounced). Does not wait for release.
    ///
    /// See the [Button trait documentation](Self) for usage examples.
    async fn wait_for_press(&mut self) {
        loop {
            <Self as __ButtonMonitor>::wait_until_pressed_state(self, false).await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if !self.is_pressed() {
                break;
            }
        }

        loop {
            <Self as __ButtonMonitor>::wait_until_pressed_state(self, true).await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if self.is_pressed() {
                break;
            }
            // otherwise it was bounce; keep waiting
        }
    }

    /// Waits for the next press and returns whether it was short or long (debounced).
    ///
    /// Returns as soon as it can decide, so long presses are reported before release.
    ///
    /// See the [Button trait documentation](Self) for usage examples.
    async fn wait_for_press_duration(&mut self) -> PressDuration {
        loop {
            <Self as __ButtonMonitor>::wait_until_pressed_state(self, false).await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if !self.is_pressed() {
                break;
            }
        }

        loop {
            <Self as __ButtonMonitor>::wait_until_pressed_state(self, true).await;
            Timer::after(BUTTON_DEBOUNCE_DELAY).await;
            if self.is_pressed() {
                break;
            }
            // otherwise it was bounce; keep waiting
        }

        let wait_for_stable_up = async {
            loop {
                <Self as __ButtonMonitor>::wait_until_pressed_state(self, false).await;
                Timer::after(BUTTON_DEBOUNCE_DELAY).await;
                if !self.is_pressed() {
                    break;
                }
            }
        };

        match select(wait_for_stable_up, Timer::after(LONG_PRESS_DURATION)).await {
            Either::First(_) => PressDuration::Short,
            Either::Second(()) => PressDuration::Long,
        }
    }
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
