//! A device abstraction for a single digital LED.
//!
//! Platform crates implement [`Led`] for their concrete single-LED device types.
//! Constructors and platform setup stay in platform crates.

use core::borrow::Borrow;

/// Logical LED level.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
pub enum LedLevel {
    /// LED on.
    On,
    /// LED off.
    #[default]
    Off,
}

/// Which GPIO pin level turns the LED on.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
pub enum OnLevel {
    /// LED lights when the pin is driven high.
    #[default]
    High,
    /// LED lights when the pin is driven low.
    Low,
}

/// Platform-agnostic single LED device contract.
///
/// This trait defines a minimal operation surface for a single digital LED:
///
/// - Set the LED level immediately with [`Led::set_level`].
/// - Run a looped animation with [`Led::animate`].
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::led::{Led, LedLevel};
/// use embassy_time::Duration;
///
/// async fn run_led_example(led: &impl Led) {
///     // Turn the LED on
///     led.set_level(LedLevel::On);
///     embassy_time::Timer::after(Duration::from_secs(1)).await;
///
///     // Turn the LED off
///     led.set_level(LedLevel::Off);
///     embassy_time::Timer::after(Duration::from_millis(500)).await;
///
///     // Play a blinking animation (looping: 200ms on, 200ms off)
///     led.animate([
///         (LedLevel::On, Duration::from_millis(200)),
///         (LedLevel::Off, Duration::from_millis(200)),
///     ]);
/// }
///
/// # struct LedSimple;
/// # impl Led for LedSimple {
/// #     fn set_level(&self, _led_level: LedLevel) {}
/// #     fn animate<I>(&self, _frames: I)
/// #     where
/// #         I: IntoIterator,
/// #         I::Item: core::borrow::Borrow<(LedLevel, embassy_time::Duration)>,
/// #     {
/// #     }
/// # }
/// # let led_simple = LedSimple;
/// # let _ = run_led_example(&led_simple);
/// ```
pub trait Led {
    /// Set the LED level immediately.
    ///
    /// See the [Led trait documentation](Self) for usage examples.
    fn set_level(&self, led_level: LedLevel);

    /// Play a looped animation sequence.
    ///
    /// The duration type is [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html).
    ///
    /// See the [Led trait documentation](Self) for usage examples.
    fn animate<I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(LedLevel, embassy_time::Duration)>;

    /// Turn the LED on.
    ///
    /// Equivalent to `set_level(LedLevel::On)`.
    fn on(&self) {
        self.set_level(LedLevel::On);
    }

    /// Turn the LED off.
    ///
    /// Equivalent to `set_level(LedLevel::Off)`.
    fn off(&self) {
        self.set_level(LedLevel::Off);
    }
}
