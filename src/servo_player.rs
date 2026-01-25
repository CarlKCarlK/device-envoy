//! A device abstraction for a servo that can animate motion sequences.
//!
//! This page provides the primary documentation and examples for controlling servos that can
//! animate motion sequences. The device abstraction supports moving to angles,
//! holding/relaxing position, and sequenced animation.
//!
//! # PWM Slices
//!
//! Supports up to eight servos, one per [PWM slice](crate#glossary). Each servo must use a
//! different slice. Calculate which slice a pin uses: `slice = pin / 2`. For example, PIN_10
//! and PIN_11 both use PWM_SLICE5 (10 / 2 = 5, 11 / 2 = 5), so only one can have a servo.
//!
//!
//! **After reading the examples below, see also:**
//!
//! - [`servo_player!`](macro@crate::servo_player) — Macro to generate a servo player struct type (includes syntax details). See [`ServoPlayerGenerated`](servo_player_generated::ServoPlayerGenerated) for a sample of a generated type.
//! - [`ServoPlayerGenerated`](servo_player_generated::ServoPlayerGenerated) — Sample struct type showing all methods and associated constants.
//! - [`combine!`](macro@crate::combine) & [`linear`] — Macro and function for creating complex motion sequences.
//! - [`Servo`] — Direct servo control without animation support.
//!
//! # Example: Basic Servo Control
//!
//! This example demonstrates basic servo control: moving to a position, holding, relaxing,
//! and using animation. Here, the generated struct type is named `ServoBasic`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_kit::{Result, servo_player::{AtEnd, servo_player}};
//! use embassy_time::{Duration, Timer};
//!
//! // Define ServoBasic, a struct type for a servo on PIN_11.
//! servo_player! {
//!     ServoBasic {
//!         pin: PIN_11,  // GPIO pin for servo PWM signal
//!         // other inputs set to their defaults
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!
//!     // PIN_11 uses PWM_SLICE5 (slice = pin / 2 = 11 / 2 = 5)
//!     let servo_basic = ServoBasic::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
//!
//!     // Move to 90°, wait 1 second, then relax.
//!     servo_basic.set_degrees(90);
//!     Timer::after(Duration::from_secs(1)).await;
//!     servo_basic.relax();
//!
//!     // Animate: hold at 180° for 1 second, then 0° for 1 second, then relax.
//!     const STEPS: [(u16, Duration); 2] = [
//!         (180, Duration::from_secs(1)),
//!         (0, Duration::from_secs(1)),
//!     ];
//!     servo_basic.animate(STEPS, AtEnd::Relax);
//!
//!     core::future::pending().await // run forever
//! }
//! ```
//!
//! # Example: Multi-Segment Animation
//!
//! This example combines multiple animation segments using the `combine!` macro to create
//! a sweep up, hold, sweep down, hold pattern. Here, the generated struct type is named `ServoWave`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_kit::{Result, combine, servo_player::{AtEnd, linear, servo_player}};
//! use embassy_time::Duration;
//!
//! // Define ServoWave, a struct type for a servo on PIN_12.
//! servo_player! {
//!     ServoWave {
//!         pin: PIN_12,
//!         max_steps: 40,  // Increase from default to hold all segments
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let servo_wave = ServoWave::new(p.PIN_12, p.PWM_SLICE6, spawner)?;
//!
//!     // Combine animation segments into one sequence.
//!     const SWEEP_UP: [(u16, Duration); 19] = linear(0, 180, Duration::from_secs(2));
//!     const HOLD_180: [(u16, Duration); 1] = [(180, Duration::from_millis(400))];
//!     const SWEEP_DOWN: [(u16, Duration); 19] = linear(180, 0, Duration::from_secs(2));
//!     const HOLD_0: [(u16, Duration); 1] = [(0, Duration::from_millis(400))];
//!     const STEPS: [(u16, Duration); 40] = combine!(SWEEP_UP, HOLD_180, SWEEP_DOWN, HOLD_0);
//!
//!     servo_wave.animate(STEPS, AtEnd::Loop);
//!
//!     core::future::pending().await // run forever
//! }
//! ```

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;
// cmk00 should led*.animate use borrow?

use core::borrow::Borrow;

use crate::servo::Servo;

/// Re-exported [`servo!`](macro@crate::servo) macro from the [`servo`](mod@crate::servo) module for convenience.
///
/// See the [`servo`](mod@crate::servo) module for direct servo control without animation.
pub use crate::servo::servo;
#[doc(hidden)]
pub use paste;

// ============================================================================
// Submodules
// ============================================================================

pub mod servo_player_generated;

/// Commands sent to the servo player device.
enum PlayerCommand<const MAX_STEPS: usize> {
    Set {
        degrees: u16,
    },
    Animate {
        steps: Vec<(u16, Duration), MAX_STEPS>,
        mode: AtEnd,
    },
    Hold,
    Relax,
}

/// Animation end behavior.
#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum AtEnd {
    /// Repeat the animation sequence indefinitely.
    Loop,
    /// Hold the final position when animation completes.
    Hold,
    /// Disable PWM after animation completes (servo relaxes).
    Relax,
}

/// Build a const linear sequence of animation steps as an array.
///
/// Returns a fixed-size array and can be used in const contexts.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::Duration;
/// # use device_kit::servo_player::linear;
/// # use panic_probe as _;
/// const SWEEP: [(u16, Duration); 11] = linear(0, 180, Duration::from_secs(2));
/// ```
#[must_use]
pub const fn linear<const N: usize>(
    start_degrees: u16,
    end_degrees: u16,
    total_duration: Duration,
) -> [(u16, Duration); N] {
    assert!(N > 0, "at least one step required");
    let step_duration = Duration::from_micros(total_duration.as_micros() / (N as u64));
    let delta = end_degrees as i32 - start_degrees as i32;
    let denom = if N == 1 { 1 } else { (N - 1) as i32 };

    let mut result = [(0u16, Duration::from_micros(0)); N];
    let mut step_index = 0;
    while step_index < N {
        let degrees = if N == 1 {
            start_degrees
        } else {
            let step_delta = delta * (step_index as i32) / denom;
            (start_degrees as i32 + step_delta) as u16
        };
        result[step_index] = (degrees, step_duration);
        step_index += 1;
    }
    result
}

/// Combine two animation step arrays into one larger array.
///
/// For combining more than two arrays, use the `combine!` macro.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::Duration;
/// # use device_kit::servo_player::{combine, linear};
/// # use panic_probe as _;
/// const SWEEP_UP: [(u16, Duration); 19] = linear(0, 180, Duration::from_secs(2));
/// const HOLD: [(u16, Duration); 1] = [(180, Duration::from_millis(400))];
/// const COMBINED: [(u16, Duration); 20] = combine(SWEEP_UP, HOLD);
/// ```
#[must_use]
#[doc(hidden)]
pub const fn combine<const N1: usize, const N2: usize, const OUT_N: usize>(
    first: [(u16, Duration); N1],
    second: [(u16, Duration); N2],
) -> [(u16, Duration); OUT_N] {
    assert!(OUT_N == N1 + N2, "OUT_N must equal N1 + N2");

    let mut result = [(0u16, Duration::from_micros(0)); OUT_N];
    let mut i = 0;
    while i < N1 {
        result[i] = first[i];
        i += 1;
    }
    let mut j = 0;
    while j < N2 {
        result[N1 + j] = second[j];
        j += 1;
    }
    result
}

/// Combine multiple animation step arrays into one larger array.
///
/// This macro allows combining any number of const arrays with a clean syntax.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::Duration;
/// # use device_kit::servo_player::linear;
/// # use device_kit::combine;
/// # use panic_probe as _;
/// const SWEEP_UP: [(u16, Duration); 19] = linear(0, 180, Duration::from_secs(2));
/// const HOLD_180: [(u16, Duration); 1] = [(180, Duration::from_millis(400))];
/// const SWEEP_DOWN: [(u16, Duration); 19] = linear(180, 0, Duration::from_secs(2));
/// const HOLD_0: [(u16, Duration); 1] = [(0, Duration::from_millis(400))];
/// const STEPS: [(u16, Duration); 40] = combine!(SWEEP_UP, HOLD_180, SWEEP_DOWN, HOLD_0);
/// ```
#[macro_export]
macro_rules! combine {
    () => {
        []
    };
    ($single:expr) => {
        $single
    };
    ($first:expr, $second:expr) => {{
        const FIRST: &[(u16, ::embassy_time::Duration)] = &$first;
        const SECOND: &[(u16, ::embassy_time::Duration)] = &$second;
        $crate::servo_player::combine::<{FIRST.len()}, {SECOND.len()}, {FIRST.len() + SECOND.len()}>($first, $second)
    }};
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        const FIRST: &[(u16, ::embassy_time::Duration)] = &$first;
        const REST: &[(u16, ::embassy_time::Duration)] = &$crate::combine!($($rest),+);
        $crate::servo_player::combine::<{FIRST.len()}, {REST.len()}, {FIRST.len() + REST.len()}>($first, $crate::combine!($($rest),+))
    }};
}

// Public so macro-generated types can reference it; hidden from docs.
#[doc(hidden)]
/// Static resources for [`ServoPlayer`].
pub struct ServoPlayerStatic<const MAX_STEPS: usize> {
    command: Signal<CriticalSectionRawMutex, PlayerCommand<MAX_STEPS>>,
}

impl<const MAX_STEPS: usize> ServoPlayerStatic<MAX_STEPS> {
    /// Create static resources for the servo player device.
    #[must_use]
    pub const fn new_static() -> Self {
        Self {
            command: Signal::new(),
        }
    }

    fn signal(&self, command: PlayerCommand<MAX_STEPS>) {
        self.command.signal(command);
    }

    async fn wait(&self) -> PlayerCommand<MAX_STEPS> {
        self.command.wait().await
    }
}

// Public so macro-generated types can deref to it; hidden from docs.
#[doc(hidden)]
/// Internal deref target for generated servo player types.
///
/// All servo player methods are available through macro-generated types.
/// See [`servo_player!`] macro documentation for usage.
pub struct ServoPlayer<const MAX_STEPS: usize> {
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
}

impl<const MAX_STEPS: usize> ServoPlayer<MAX_STEPS> {
    /// Create static resources for a servo player.
    #[must_use]
    pub const fn new_static() -> ServoPlayerStatic<MAX_STEPS> {
        ServoPlayerStatic::new_static()
    }

    /// Create a servo player handle. The device loop must already be running.
    ///
    /// See the [servo_player module documentation](mod@crate::servo_player) for usage.
    #[must_use]
    pub const fn new(servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>) -> Self {
        Self {
            servo_player_static,
        }
    }

    /// Set the target angle. The most recent command always wins.
    ///
    /// See the [servo_player module documentation](mod@crate::servo_player) for usage.
    pub fn set_degrees(&self, degrees: u16) {
        self.servo_player_static
            .signal(PlayerCommand::Set { degrees });
    }

    /// Hold the servo at its current position (turn on PWM).
    ///
    /// See the [servo_player module documentation](mod@crate::servo_player) for usage.
    pub fn hold(&self) {
        self.servo_player_static.signal(PlayerCommand::Hold);
    }

    /// Relax the servo (turn off PWM, servo can move freely).
    ///
    /// See the [servo_player module documentation](mod@crate::servo_player) for usage.
    pub fn relax(&self) {
        self.servo_player_static.signal(PlayerCommand::Relax);
    }

    /// Animate the servo through a sequence of angles with per-step hold durations.
    ///
    /// Each step is a tuple `(degrees, duration)`. Accepts both owned iterators and
    /// references to collections.
    ///
    /// See the [servo_player module documentation](mod@crate::servo_player) for usage.
    pub fn animate<I>(&self, steps: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<(u16, Duration)>,
    {
        assert!(MAX_STEPS > 0, "animate disabled: max_steps is 0");
        let mut sequence: Vec<(u16, Duration), MAX_STEPS> = Vec::new();
        for step in steps {
            let step = *step.borrow();
            assert!(
                step.1.as_micros() > 0,
                "animation step duration must be positive"
            );
            sequence
                .push(step)
                .expect("animate sequence fits within max_steps");
        }
        assert!(!sequence.is_empty(), "animate requires at least one step");

        self.servo_player_static.signal(PlayerCommand::Animate {
            steps: sequence,
            mode: at_end,
        });
    }
}

/// Macro to generate a servo player struct type (includes syntax details).
///
/// This page provides the primary documentation for configuring individual servo players.
///
/// **After reading the configuration details below, see also:**
///
/// - [`ServoPlayerGenerated`](servo_player_generated::ServoPlayerGenerated) — Sample servo player type showing all methods and associated constants
/// - [`servo_player`](mod@crate::servo_player) module — Complete examples and usage patterns
/// - [`ServoPlayer`] — Core player type used by macro-generated types
///
/// Use this macro when your project has a servo that needs scripted animation control.
/// The macro generates a struct type that wraps [`ServoPlayer`] and spawns a background
/// task to execute animation sequences.
///
/// # Configuration
///
/// ## Required Fields
///
/// - `pin` — GPIO pin for servo PWM signal
///
/// ## Optional Fields
///
/// - `min_us` — Minimum pulse width in microseconds for 0° (default: 500)
/// - `max_us` — Maximum pulse width in microseconds for max_degrees (default: 2500)
/// - `max_degrees` — Maximum servo angle in degrees (default: 270)
/// - `max_steps` — Maximum number of animation steps (default: 16)
///
/// `max_steps = 0` disables animation and allocates no step storage; `set_degrees()`, `hold()`, and `relax()` are still supported.
///
/// # Example
///
/// See the [servo_player module documentation](mod@crate::servo_player) for complete examples.
///
/// Basic usage:
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_kit::servo_player::servo_player;
///
/// servo_player! {
///     ServoSweep {
///         pin: PIN_11,  // Required: GPIO pin for servo PWM
///     }
/// }
/// # fn main() {}
/// ```
///
/// With custom configuration:
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_kit::servo_player::servo_player;
///
/// servo_player! {
///     pub(self) ServoCustom {     // Can provide a visibility modifier
///         pin: PIN_12,            // GPIO pin for servo PWM
///         min_us: 544,            // Minimum pulse width (µs)
///         max_us: 2400,           // Maximum pulse width (µs)
///         max_degrees: 180,       // Maximum angle (degrees)
///         max_steps: 40,          // Maximum animation steps
///     }
/// }
/// # fn main() {}
/// ```
#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! servo_player {
    ($($tt:tt)*) => { $crate::__servo_player_impl! { $($tt)* } };
}
#[doc(inline)]
pub use servo_player;

// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_player_impl {
    // Entry point - name without visibility defaults to public
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: pub,
            name: $name,
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::Servo::DEFAULT_MAX_DEGREES,
            max_steps: 16,
            fields: [ $($fields)* ]
        }
    };

    // Entry point - name with explicit visibility
    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::Servo::DEFAULT_MAX_DEGREES,
            max_steps: 16,
            fields: [ $($fields)* ]
        }
    };

    // Fill defaults: pin
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ pin: $pin_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin_value,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ pin: $pin_value:ident ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin_value,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: slice
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ slice: $slice_value:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice_value,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ slice: $slice_value:ident ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice_value,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: min_us
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ min_us: $min_us_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us_value,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ min_us: $min_us_value:expr ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us_value,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: max_us
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_us: $max_us_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us_value,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_us: $max_us_value:expr ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us_value,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: max_degrees
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_degrees: $max_degrees_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees_value,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_degrees: $max_degrees_value:expr ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees_value,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: max_steps
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_steps: $max_steps_value:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps_value,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ max_steps: $max_steps_value:expr ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps_value,
            fields: [ ]
        }
    };

    // Fill defaults: channel overrides
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ channel: A $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ channel: A ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ channel: B $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ channel: B ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ even $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ even ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ odd $(, $($rest:tt)* )? ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ odd ]
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps,
            fields: [ ]
        }
    };

    // Fill defaults: terminate and build
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr,
        fields: [ ]
    ) => {
        $crate::__servo_player_impl! {
            @__build
            vis: $vis,
            name: $name,
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            max_steps: $max_steps
        }
    };

    // Build errors for missing fields
    (@__build
        vis: $vis:vis,
        name: $name:ident,
        pin: _UNSET_,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        compile_error!("servo_player! requires `pin: ...`");
    };

    // Build with all fields set (slice can be _UNSET_ - it's in the new() signature)
    (@__build
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:ident,
        slice: _UNSET_,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        $crate::servo_player::paste::paste! {
            static [<$name:upper _SERVO_PLAYER_STATIC>]: $crate::servo_player::ServoPlayerStatic<$max_steps> =
                $crate::servo_player::ServoPlayer::<$max_steps>::new_static();
            static [<$name:upper _SERVO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            $vis struct $name {
                player: $crate::servo_player::ServoPlayer<$max_steps>,
            }

            impl $name {
                /// Create the servo player and spawn its background task.
                ///
                /// The slice is automatically determined from the pin via the type system.
                ///
                /// See the [struct-level example](Self) for usage.
                pub fn new<S: 'static>(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    slice: impl Into<::embassy_rp::Peri<'static, S>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self>
                where
                    ::embassy_rp::peripherals::$pin: $crate::servo::ServoPwmPin<S>,
                    S: ::embassy_rp::PeripheralType,
                {
                    let pin = pin.into();
                    let slice = slice.into();
                    let servo = $crate::servo::servo_from_pin_slice(
                        pin,
                        slice,
                        $min_us,
                        $max_us,
                        $max_degrees
                    );
                    let token = [<$name:snake _servo_player_task>](&[<$name:upper _SERVO_PLAYER_STATIC>], servo);
                    spawner.spawn(token)?;
                    let player = $crate::servo_player::ServoPlayer::new(&[<$name:upper _SERVO_PLAYER_STATIC>]);
                    Ok([<$name:upper _SERVO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::servo_player::ServoPlayer<$max_steps>;

                fn deref(&self) -> &Self::Target {
                    &self.player
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo_player::ServoPlayerStatic<$max_steps>,
                servo: $crate::servo::Servo<'static>,
            ) -> ! {
                $crate::servo_player::device_loop(servo_player_static, servo).await
            }
        }
    };

    (@__build
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:ident,
        slice: $slice:ident,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        $crate::servo_player::paste::paste! {
            static [<$name:upper _SERVO_PLAYER_STATIC>]: $crate::servo_player::ServoPlayerStatic<$max_steps> =
                $crate::servo_player::ServoPlayer::<$max_steps>::new_static();
            static [<$name:upper _SERVO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            $vis struct $name {
                player: $crate::servo_player::ServoPlayer<$max_steps>,
            }

            impl $name {
                /// Create the servo player and spawn its background task.
                ///
                /// See the [struct-level example](Self) for usage.
                pub fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    slice: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$slice>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    let pin = pin.into();
                    let slice = slice.into();
                    let servo = $crate::__servo_player_impl! {
                        @__build_servo
                        pin: pin,
                        slice: slice,
                        channel: $channel,
                        min_us: $min_us,
                        max_us: $max_us,
                        max_degrees: $max_degrees
                    };
                    let token = [<$name:snake _servo_player_task>](&[<$name:upper _SERVO_PLAYER_STATIC>], servo);
                    spawner.spawn(token)?;
                    let player = $crate::servo_player::ServoPlayer::new(&[<$name:upper _SERVO_PLAYER_STATIC>]);
                    Ok([<$name:upper _SERVO_PLAYER_CELL>].init(Self { player }))
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::servo_player::ServoPlayer<$max_steps>;

                fn deref(&self) -> &Self::Target {
                    &self.player
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo_player::ServoPlayerStatic<$max_steps>,
                servo: $crate::servo::Servo<'static>,
            ) -> ! {
                $crate::servo_player::device_loop(servo_player_static, servo).await
            }
        }
    };

    (@__build_servo
        pin: $pin:expr,
        slice: $slice:expr,
        channel: _UNSET_,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        $crate::servo::servo_from_pin_slice($pin, $slice, $min_us, $max_us, $max_degrees)
    };

    (@__build_servo
        pin: $pin:expr,
        slice: $slice:expr,
        channel: A,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        $crate::servo::Servo::new_output_a(
            embassy_rp::pwm::Pwm::new_output_a(
                $slice,
                $pin,
                embassy_rp::pwm::Config::default(),
            ),
            $min_us,
            $max_us,
            $max_degrees,
        )
    };

    (@__build_servo
        pin: $pin:expr,
        slice: $slice:expr,
        channel: B,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        max_steps: $max_steps:expr
    ) => {
        $crate::servo::Servo::new_output_b(
            embassy_rp::pwm::Pwm::new_output_b(
                $slice,
                $pin,
                embassy_rp::pwm::Config::default(),
            ),
            $min_us,
            $max_us,
            $max_degrees,
        )
    };

    (
        $($fields:tt)*
    ) => {
        $crate::__servo_player_impl! {
            @__fill_defaults
            vis: pub,
            name: ServoPlayerGenerated,
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::Servo::DEFAULT_MAX_DEGREES,
            max_steps: 16,
            fields: [ $($fields)* ]
        }
    };
}

// Called by macro-generated code in downstream crates; must be public.
#[doc(hidden)]
pub async fn device_loop<const MAX_STEPS: usize>(
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
    mut servo: Servo<'static>,
) -> ! {
    let mut current_degrees: u16 = 0;
    servo.set_degrees(current_degrees);

    let mut command = servo_player_static.wait().await;
    loop {
        match command {
            PlayerCommand::Set { degrees } => {
                current_degrees = degrees;
                servo.set_degrees(current_degrees);
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Hold => {
                servo.hold();
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Relax => {
                servo.relax();
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Animate { steps, mode } => {
                command = run_animation(
                    &steps,
                    mode,
                    &mut servo,
                    servo_player_static,
                    &mut current_degrees,
                )
                .await;
            }
        }
    }
}

async fn run_animation<const MAX_STEPS: usize>(
    steps: &[(u16, Duration)],
    mode: AtEnd,
    servo: &mut Servo<'static>,
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
    current_degrees: &mut u16,
) -> PlayerCommand<MAX_STEPS> {
    loop {
        for step in steps {
            if *current_degrees != step.0 {
                servo.set_degrees(step.0);
                *current_degrees = step.0;
            }
            match select(Timer::after(step.1), servo_player_static.wait()).await {
                Either::First(_) => {}
                Either::Second(command) => return command,
            }
        }

        // Animation sequence completed - handle end behavior
        match mode {
            AtEnd::Loop => {
                // Continue looping
            }
            AtEnd::Hold => {
                // Hold final position and wait for next command
                return servo_player_static.wait().await;
            }
            AtEnd::Relax => {
                // Disable PWM (servo relaxes) and wait for next command
                servo.relax();
                return servo_player_static.wait().await;
            }
        }
    }
}
