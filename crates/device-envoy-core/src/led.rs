//! A device abstraction for a single digital LED.
//!
//! Platform crates implement [`Led`] for their concrete single-LED device types.
//! Constructors and platform setup stay in platform crates.

use core::borrow::Borrow;

use crate::UnwrapInfallible;

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

// A shared reference to a device is the device: it lets code that takes the trait by value
// accept the `&'static` handles that the declaration macros return.
impl<T> Led for &T
where
    T: Led + ?Sized,
{
    fn set_level(&self, led_level: LedLevel) {
        (**self).set_level(led_level);
    }

    fn animate<I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(LedLevel, embassy_time::Duration)>,
    {
        (**self).animate(frames);
    }

    fn on(&self) {
        (**self).on();
    }

    fn off(&self) {
        (**self).off();
    }
}

// ============================================================================
// Shared runtime for the `led!` macro on every chip.
// Public because the macro expands in downstream crates; hidden from user docs.
// ============================================================================

/// Default animation length for `led!`.
#[doc(hidden)]
pub const DEFAULT_MAX_STEPS: usize = 32;

/// A command sent from an LED handle to its background task.
#[derive(Clone)]
#[doc(hidden)]
pub enum LedCommand<const MAX_STEPS: usize> {
    Set(LedLevel),
    Animate(heapless::Vec<(LedLevel, embassy_time::Duration), MAX_STEPS>),
}

/// Carries commands from an LED handle to its background task.
#[doc(hidden)]
pub type LedOuterStatic<const MAX_STEPS: usize> = embassy_sync::signal::Signal<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    LedCommand<MAX_STEPS>,
>;

/// The static state behind one `led!` device.
#[doc(hidden)]
pub struct LedStatic<const MAX_STEPS: usize> {
    outer: LedOuterStatic<MAX_STEPS>,
}

impl<const MAX_STEPS: usize> LedStatic<MAX_STEPS> {
    #[doc(hidden)]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            outer: embassy_sync::signal::Signal::new(),
        }
    }

    #[doc(hidden)]
    pub fn outer(&self) -> &LedOuterStatic<MAX_STEPS> {
        &self.outer
    }
}

/// Drives `pin` to show `led_level`, given which pin level lights the LED.
#[doc(hidden)]
pub fn set_pin_for_led_level(
    led_level: LedLevel,
    pin: &mut impl embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
    on_level: OnLevel,
) {
    match (led_level, on_level) {
        (LedLevel::On, OnLevel::High) | (LedLevel::Off, OnLevel::Low) => {
            pin.set_high().unwrap_infallible();
        }
        (LedLevel::Off, OnLevel::High) | (LedLevel::On, OnLevel::Low) => {
            pin.set_low().unwrap_infallible();
        }
    }
}

/// Holds `led_level` until a different command arrives, and returns that command.
#[doc(hidden)]
pub async fn run_set_level_loop<const MAX_STEPS: usize>(
    led_level: LedLevel,
    outer_static: &'static LedOuterStatic<MAX_STEPS>,
    pin: &mut impl embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
    on_level: OnLevel,
) -> LedCommand<MAX_STEPS> {
    set_pin_for_led_level(led_level, pin, on_level);

    loop {
        match outer_static.wait().await {
            LedCommand::Set(new_led_level) => {
                if new_led_level == led_level {
                    continue;
                }
                return LedCommand::Set(new_led_level);
            }
            other => return other,
        }
    }
}

/// Loops `animation` until a new command arrives, and returns that command.
#[doc(hidden)]
pub async fn run_animation_loop<const MAX_STEPS: usize>(
    animation: heapless::Vec<(LedLevel, embassy_time::Duration), MAX_STEPS>,
    outer_static: &'static LedOuterStatic<MAX_STEPS>,
    pin: &mut impl embedded_hal::digital::OutputPin<Error = core::convert::Infallible>,
    on_level: OnLevel,
) -> LedCommand<MAX_STEPS> {
    if animation.is_empty() {
        return LedCommand::Animate(animation);
    }

    let mut frame_index = 0;

    loop {
        let (led_level, duration) = animation[frame_index];

        set_pin_for_led_level(led_level, pin, on_level);

        frame_index = (frame_index + 1) % animation.len();

        match embassy_futures::select::select(
            embassy_time::Timer::after(duration),
            outer_static.wait(),
        )
        .await
        {
            embassy_futures::select::Either::First(_) => {}
            embassy_futures::select::Either::Second(command) => return command,
        }
    }
}
