//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides two ways to monitor button presses:
//!
//! - [`Button`] — Simple button monitoring. Each call to `wait_for_press()`, etc. starts fresh
//!   button monitoring.
//! - [`button_watch!`](crate::button_watch!) — Monitors a button in a background task
//!   so that it works even in a fast loop/select.
//! 

mod button_watch;
pub mod button_watch_generated;

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub use button_watch::{ButtonWatch, ButtonWatchStatic};

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub use button_watch::{button_watch_task, button_watch_task_from_input};

use embassy_futures::select::{Either, select};
use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pull};
use embassy_time::{Duration, Timer};

// ============================================================================
// Constants
// ============================================================================

/// Debounce delay for the button.
pub(crate) const BUTTON_DEBOUNCE_DELAY: Duration = Duration::from_millis(10);

/// Duration representing a long button press.
pub(crate) const LONG_PRESS_DURATION: Duration = Duration::from_millis(500);

// ============================================================================
// PressedTo - How the button is wired
// ============================================================================

/// Describes if the button connects to voltage or ground when pressed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, defmt::Format)]
pub enum PressedTo {
    /// Button connects pin to voltage (3.3V) when pressed.
    /// Uses internal pull-down resistor. Pin reads HIGH when pressed.
    ///
    /// Note: The original Pico 2 (RP2350) has a known silicon bug with pull-down resistors
    /// that can cause pins to stay HIGH after button release. Use ToGround instead.
    Voltage,

    /// Button connects pin to ground (GND) when pressed.
    /// Uses internal pull-up resistor. Pin reads LOW when pressed.
    /// Recommended for Pico 2 due to pull-down resistor bug.
    Ground,
}

// ============================================================================
// PressDuration - Button press type
// ============================================================================

/// Duration of a button press (short or long).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, defmt::Format)]
pub enum PressDuration {
    /// Button was held for less than [`LONG_PRESS_DURATION`](crate::button) (500ms).
    Short,
    /// Button was held for at least [`LONG_PRESS_DURATION`](crate::button) (500ms).
    Long,
}

// ============================================================================
// Button Virtual Device
// ============================================================================

/// A device abstraction for a button with debouncing and press duration detection.
///
/// # Hardware Requirements
///
/// The button can be wired in two ways:
/// - [`PressedTo::Voltage`]: Button connects pin to 3.3V when pressed (uses pull-down)
/// - [`PressedTo::Ground`]: Button connects pin to GND when pressed (uses pull-up)
///
/// **Important**: Pico 2 (RP2350) has a known silicon bug (erratum E9) with pull-down
/// resistors that can leave the pin reading HIGH after release. Wire buttons to GND and
/// use [`PressedTo::Ground`] on Pico 2.
///
/// # Usage
///
/// Use [`wait_for_press()`](Self::wait_for_press) when you only need a debounced
/// press event. It returns on the down edge and does not wait for release.
///
/// Use [`wait_for_press_duration()`](Self::wait_for_press_duration) when you need to
/// distinguish short vs. long presses. It returns as soon as it can decide, so long
/// presses are reported before the button is released.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
///
/// use device_envoy::button::{Button, PressDuration, PressedTo};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(p: embassy_rp::Peripherals) {
///     let mut button = Button::new(p.PIN_13, PressedTo::Ground);
///
///     // Wait for a press without measuring duration.
///     button.wait_for_press().await;
///
///     // Measure press durations in a loop
///     loop {
///         match button.wait_for_press_duration().await {
///             PressDuration::Short => {
///                 // Handle short press
///             }
///             PressDuration::Long => {
///                 // Handle long press (fires before button is released)
///             }
///         }
///     }
/// }
/// ```
pub struct Button<'a> {
    input: Input<'a>,
    pressed_to: PressedTo,
}

impl<'a> Button<'a> {
    /// Creates a new `Button` instance from a pin.
    ///
    /// The pin is configured based on the connection type:
    /// - [`PressedTo::Voltage`]: Uses internal pull-down (button to 3.3V)
    /// - [`PressedTo::Ground`]: Uses internal pull-up (button to GND)
    #[must_use]
    pub fn new<P: embassy_rp::gpio::Pin>(pin: Peri<'a, P>, pressed_to: PressedTo) -> Self {
        let pull = match pressed_to {
            PressedTo::Voltage => Pull::Down,
            PressedTo::Ground => Pull::Up,
        };
        Self {
            input: Input::new(pin, pull),
            pressed_to,
        }
    }

    /// Returns whether the button is currently pressed.
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        match self.pressed_to {
            PressedTo::Voltage => self.input.is_high(),
            PressedTo::Ground => self.input.is_low(),
        }
    }

    #[inline]
    async fn wait_for_button_up(&mut self) -> &mut Self {
        loop {
            if !self.is_pressed() {
                break;
            }
            Timer::after(Duration::from_millis(1)).await;
        }
        self
    }

    #[inline]
    async fn wait_for_button_down(&mut self) -> &mut Self {
        loop {
            if self.is_pressed() {
                break;
            }
            Timer::after(Duration::from_millis(1)).await;
        }
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
    /// Waits for the next press (button goes down, debounced).
    /// Does not wait for release.
    ///
    /// See [`Button`] for usage example
    pub async fn wait_for_press(&mut self) {
        self.wait_for_stable_up().await; // ensure edge-triggered
        self.wait_for_stable_down().await; // return on down
    }

    /// Waits for the next press and returns whether it was short or long (debounced).
    ///
    /// Returns as soon as it can decide, so long presses are reported before release.
    ///
    /// See [`Button`] for usage example
    pub async fn wait_for_press_duration(&mut self) -> PressDuration {
        self.wait_for_stable_up().await;
        self.wait_for_stable_down().await;

        let press_duration =
            match select(self.wait_for_stable_up(), Timer::after(LONG_PRESS_DURATION)).await {
                Either::First(_) => PressDuration::Short,
                Either::Second(()) => PressDuration::Long,
            };

        press_duration
    }

    /// Waits until the button is released (debounced).
    pub async fn wait_for_release(&mut self) {
        self.wait_for_stable_up().await;
    }

    /// Consumes the button and returns its internal components.
    ///
    /// This is useful for converting a `Button` (returned from `WifiAuto::connect`)
    /// into a `ButtonWatch` for background monitoring.
    ///
    /// See the [`button_watch!`](crate::button_watch!) macro documentation for usage with `from_button()`.
    // Must be public for macro expansion but not part of the user-facing API.
    #[doc(hidden)]
    #[must_use]
    pub fn into_parts(self) -> (Input<'a>, PressedTo) {
        (self.input, self.pressed_to)
    }
}

#[doc(inline)]
pub use crate::button_watch;
