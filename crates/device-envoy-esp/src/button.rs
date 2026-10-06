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
#[doc(hidden)]
pub use device_envoy_core::button::__ButtonMonitor;
pub use device_envoy_core::button::Button;
pub use device_envoy_core::button::{PressDuration, PressedTo};
// Public for compatibility; hidden from end-user docs.
#[doc(hidden)]
pub use device_envoy_core::button::{
    BUTTON_DEBOUNCE_DELAY, BUTTON_POLL_INTERVAL, LONG_PRESS_DURATION,
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

const_structures::define! {
    #[cfg(target_os = "none")]
    /// Creates a button monitoring device abstraction with a background task.
    ///
    /// This macro creates a button monitor that runs in a dedicated background task,
    /// providing continuous monitoring without interruption.
    ///
    /// See [`ButtonWatchGenerated`](crate::button::button_watch_generated::ButtonWatchGenerated)
    /// for a sample of what the macro generates.
    ///
    /// # Constructors
    ///
    /// - [`new()`](crate::button::button_watch_generated::ButtonWatchGenerated::new) — Create from a pin
    ///
    /// # Use Cases
    ///
    /// Use `button_watch!` instead of [`ButtonEsp`] when you need continuous monitoring
    /// that works even in fast loops or `select()` operations. [`ButtonEsp`] starts
    /// fresh monitoring on each call to `wait_for_press()`, which can miss events in busy loops.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #![no_std]
    /// # #![no_main]
    /// use device_envoy_esp::{
    ///     Result,
    ///     button::{Button as _, PressDuration, PressedTo},
    ///     button_watch,
    /// };
    /// use embassy_executor::Spawner;
    /// # use esp_backtrace as _;
    /// # #[panic_handler]
    /// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
    ///
    /// button_watch! {
    ///     ButtonWatch13 {
    ///         pin: GPIO13,
    ///     }
    /// }
    ///
    /// async fn example(
    ///     p: esp_hal::peripherals::Peripherals,
    ///     spawner: Spawner,
    /// ) -> Result<()> {
    ///     // Create the button monitor (spawns background task automatically)
    ///     let mut button_watch13 = ButtonWatch13::new(p.GPIO13, PressedTo::Ground, spawner)
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
    pub button_watch {
        /// GPIO pin connected to the button, for example `GPIO6`.
        pin: ident,
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        #[doc = "\n\nMonitors button presses in a background task. See the [button module documentation](mod@device_envoy_esp::button) for usage."]
        $decl.vis struct $decl.name {
            button_watch: $crate::button::ButtonWatchEsp<'static>,
        }

        impl $decl.name {
            /// Creates a new button monitor and spawns its background task.
            ///
            /// # Parameters
            ///
            /// - `button_pin`: GPIO pin for the button
            /// - `pressed_to`: How the button is wired ([`PressedTo::Ground`] or [`PressedTo::Voltage`])
            /// - `spawner`: Task spawner for background operations
            ///
            /// # Errors
            ///
            /// Returns an error if the background task cannot be spawned.
            pub async fn new(
                button_pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                pressed_to: $crate::button::PressedTo,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static mut Self> {
                static BUTTON_WATCH_STATIC: $crate::button::ButtonWatchStaticEsp =
                    $crate::button::ButtonWatchEsp::new_static();
                static BUTTON_WATCH_CELL: ::static_cell::StaticCell<$decl.name> =
                    ::static_cell::StaticCell::new();

                let button_watch = $crate::button::ButtonWatchEsp::new_from_pin(
                    &BUTTON_WATCH_STATIC,
                    button_pin,
                    pressed_to,
                    spawner,
                )
                .await?;

                let instance = BUTTON_WATCH_CELL.init($decl.name { button_watch });
                Ok(instance)
            }
        }

        impl ::core::ops::Deref for $decl.name {
            type Target = $crate::button::ButtonWatchEsp<'static>;

            fn deref(&self) -> &Self::Target {
                &self.button_watch
            }
        }

        impl $crate::button::__ButtonMonitor for $decl.name {
            fn is_pressed_raw(&self) -> bool {
                <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::is_pressed(
                    &self.button_watch,
                )
            }

            async fn wait_until_pressed_state(&mut self, pressed: bool) {
                <$crate::button::ButtonWatchEsp<'static> as $crate::button::__ButtonMonitor>::wait_until_pressed_state(
                    &mut self.button_watch,
                    pressed,
                )
                .await
            }
        }

        impl $crate::button::Button for $decl.name {
            async fn wait_for_press_duration(&mut self) -> $crate::button::PressDuration {
                <$crate::button::ButtonWatchEsp<'static> as $crate::button::Button>::wait_for_press_duration(
                    &mut self.button_watch,
                )
                .await
            }
        }
    }
}
