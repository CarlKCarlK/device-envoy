//! Platform-independent button types and constants.
//!
//! See the platform-specific crate (for example `device_envoy_rp::button` or
//! `device_envoy_esp::button`) for the primary documentation and examples.

use embassy_time::Duration;

// ============================================================================
// Constants
// ============================================================================

/// Debounce delay for the button.
pub const BUTTON_DEBOUNCE_DELAY: Duration = Duration::from_millis(10);

/// Duration representing a long button press.
pub const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

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
