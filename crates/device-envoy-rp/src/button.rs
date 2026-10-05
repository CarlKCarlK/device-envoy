//! A device abstraction for buttons with debouncing and press duration detection.
//!
//! This module provides two ways to monitor button presses:
//!
//! - [`ButtonRp`] — Simple button monitoring. Each call to `wait_for_press()`, etc. starts fresh
//!   button monitoring.
//! - [`button_watch!`](crate::button_watch!) — Monitors a button in a background task
//!   so that it works even in a fast loop/select.
//!

mod button_watch;
pub mod button_watch_generated;

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub use button_watch::{ButtonWatchRp, ButtonWatchStaticRp};

// Must be public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
pub use button_watch::button_watch_task;

#[doc(hidden)]
pub use device_envoy_core::button::__ButtonMonitor;
pub use device_envoy_core::button::Button;
pub use device_envoy_core::button::{PressDuration, PressedTo};
// Public for compatibility; hidden from end-user docs.
#[doc(hidden)]
pub use device_envoy_core::button::{
    BUTTON_DEBOUNCE_DELAY, BUTTON_POLL_INTERVAL, LONG_PRESS_DURATION,
};

use embassy_rp::Peri;
use embassy_rp::gpio::{Input, Pull};

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
/// use device_envoy_rp::button::{Button as _, ButtonRp, PressDuration, PressedTo};
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(p: embassy_rp::Peripherals) {
///     let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
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
pub struct ButtonRp<'a> {
    input: Input<'a>,
    pressed_to: PressedTo,
}

impl<'a> ButtonRp<'a> {
    /// Creates a new `ButtonRp` instance from a pin.
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
}

impl device_envoy_core::button::__ButtonMonitor for ButtonRp<'_> {
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

impl device_envoy_core::button::Button for ButtonRp<'_> {}

const_structures::define! {
    /// Creates a button monitoring device abstraction with a background task.
    ///
    /// This macro creates a button monitor that runs in a dedicated background task,
    /// providing continuous monitoring without interruption.
    ///
    /// See [`ButtonWatchGenerated`](crate::button::button_watch_generated::ButtonWatchGenerated) for a sample of what the macro generates.
    ///
    /// # Constructors
    ///
    /// - [`new()`](crate::button::button_watch_generated::ButtonWatchGenerated::new) — Create from a pin
    ///
    /// # Use Cases
    ///
    /// Use `button_watch!` instead of [`ButtonRp`] when you need continuous monitoring
    /// that works even in fast loops or `select()` operations. [`ButtonRp`] starts
    /// fresh monitoring on each call to `wait_for_press()`, which can miss events in busy loops.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// use device_envoy_rp::button_watch;
    /// use device_envoy_rp::button::PressDuration;
    /// use device_envoy_rp::button::PressedTo;
    /// use device_envoy_rp::button::Button as _;
    /// use embassy_executor::Spawner;
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    ///
    /// button_watch! {
    ///     ButtonWatch13 {
    ///         pin: PIN_13,
    ///     }
    /// }
    ///
    /// async fn example(
    ///     p: embassy_rp::Peripherals,
    ///     spawner: Spawner,
    /// ) -> device_envoy_rp::Result<()> {
    ///     // Create the button monitor (spawns background task automatically)
    ///     let mut button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner)
    ///         .await?;
    ///
    ///     loop {
    ///         // Wait for button press - never misses events even if this loop is slow
    ///         match button_watch13.wait_for_press_duration().await {
    ///             PressDuration::Short => {
    ///                 // Handle short press
    /// #               break;
    ///             }
    ///             PressDuration::Long => {
    ///                 // Handle long press
    /// #               break;
    ///             }
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub button_watch => __button_watch_generate {
        /// GPIO pin connected to the button, for example `PIN_13`.
        pin: ident,
    }
}
