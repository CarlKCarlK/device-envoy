//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides two ways to monitor button presses:
//!
//! - [`ButtonEsp`] - Simple button monitoring. Each call to `wait_for_press()`, etc. starts fresh
//!   button monitoring.
//! - [`button_watch!`](crate::button_watch!) - Monitors a button in a background task
//!   so that it works even in a fast loop/select.

#[cfg(target_os = "none")]
mod button_watch;
pub mod button_watch_generated;

#[cfg(target_os = "none")]
// `button_watch!` expands in downstream crates and references these symbols, so
// they must remain public even though they are macro implementation details.
#[doc(hidden)]
pub use button_watch::{ButtonWatchEsp, ButtonWatchStaticEsp};
pub use device_envoy_core::button::Button;
#[doc(hidden)]
pub use device_envoy_core::button::__ButtonMonitor;
pub use device_envoy_core::button::{
    PressDuration, PressedTo, BUTTON_DEBOUNCE_DELAY, BUTTON_POLL_INTERVAL, LONG_PRESS_DURATION,
};

#[cfg(target_os = "none")]
use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

/// A device abstraction for a button with debouncing and press duration detection.
///
/// # Hardware Requirements
///
/// The button can be wired in two ways:
/// - [`PressedTo::Voltage`]: Button connects pin to 3.3V when pressed (uses pull-down)
/// - [`PressedTo::Ground`]: Button connects pin to GND when pressed (uses pull-up)
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
/// # #![no_std]
/// # #![no_main]
///
/// use device_envoy_esp::{
///     button::{Button as _, ButtonEsp, PressDuration, PressedTo},
///     init_and_start, Result,
/// };
/// # use core::convert::Infallible;
/// # use esp_backtrace as _;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example() -> Result<Infallible> {
///     init_and_start!(p);
///     let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
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
}

#[cfg(target_os = "none")]
impl device_envoy_core::button::__ButtonMonitor for ButtonEsp<'_> {
    fn is_pressed_raw(&self) -> bool {
        self.pressed_to.is_pressed(self.input.is_high())
    }

    // todo0 test this. It used to poll.
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
impl device_envoy_core::button::Button for ButtonEsp<'_> {}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use crate::button_watch;
