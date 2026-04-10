//! A device abstraction for hobby servos that can animate motion sequences.
//!
//! This page provides the primary documentation and examples for controlling servos that can
//! animate motion sequences. The device abstraction supports moving to angles,
//! holding/relaxing position, and sequenced animation.
//! [`ServoPlayer`](crate::servo::ServoPlayer) extends [`Servo`](crate::servo::Servo), so
//! servo player types support `set_degrees`, `hold`, and `relax` plus animation. Depending
//! on method-resolution context, call sites may still need [`Servo`](crate::servo::Servo)
//! in scope.
//!
//! **After reading the examples below, see also:**
//!
//! - [`servo_player!`](macro@crate::servo::servo_player) — Macro to generate a servo player struct
//!   type (includes syntax details).
//! - [`combine!`](macro@crate::servo::combine) & [`linear`] — Macro and function for creating
//!   complex motion sequences.
//! - [`servo!`](macro@crate::servo::servo) — Direct servo control without animation support.

#![doc = include_str!("../docs/how_servos_work.md")]

//!
//! This device abstraction, `servo_player`, adds a background software task around the hardware
//! control signal.
//!
//! # Controlling Multiple Servos
//!
//! Supports up to eight servos, one per [PWM slice](crate#glossary) resource. To calculate which PWM slice a pin uses,
//! use the formula: `PWM slice = (pin / 2) % 8`. For example, PIN_10 and PIN_11 must both use PWM_SLICE5
//! ((10 / 2) % 8 = 5, (11 / 2) % 8 = 5). Therefore, either of these these two pins can have a servo, but not both.
//!
//!
//! # Example: Basic Servo Control
//!
//! This example demonstrates basic servo control: moving to a position, holding, relaxing,
//! and using animation. Here, the generated struct type is named `ServoPlayer11`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{Result, servo::{AtEnd, Servo as _, ServoPlayer as _, servo_player}};
//! use embassy_time::{Duration, Timer};
//!
//! // Define ServoPlayer11, a struct type for a servo on PIN_11.
//! servo_player! {
//!     ServoPlayer11 {
//!         pin: PIN_11,  // GPIO pin for servo
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
//!     // PIN_11 uses PWM_SLICE5 (pin / 2) % 8 = (11 / 2) % 8 = 5 % 8 = 5)
//!     let servo_player11 = ServoPlayer11::new(p.PIN_11, p.PWM_SLICE5, spawner)?;
//!
//!     // Move to 90°, wait 1 second, then relax.
//!     servo_player11.set_degrees(90);
//!     Timer::after(Duration::from_secs(1)).await;
//!     servo_player11.relax();
//!
//!     // Animate: hold at 180° for 1 second, then 0° for 1 second, then relax.
//!     const STEPS: [(u16, Duration); 2] = [
//!         (180, Duration::from_secs(1)),
//!         (0, Duration::from_secs(1)),
//!     ];
//!     // AtEnd::Relax quiets the servo; AtEnd::Hold keeps driving pulses to hold
//!     // position; AtEnd::Loop repeats.
//!     servo_player11.animate(STEPS, AtEnd::Relax);
//!
//!     pending().await // run forever
//! }
//! ```
//!
//! # Example: Multi-Step Animation
//!
//! This example combines 40 animation steps using `linear` and`combine!` to
//! sweep up, hold, sweep down, hold pattern. Here, the generated struct type is named
//! `ServoSweep`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{Result, servo::{AtEnd, Servo as _, ServoPlayer as _, combine, linear, servo_player}};
//! use embassy_time::Duration;
//!
//! // Define ServoSweep, a struct type for a servo on PIN_12.
//! servo_player! {
//!     ServoSweep {
//!         pin: PIN_12,
//!         max_steps: 40,          // Increase from default (16) to hold animation steps
//!
//!        // Optional
//!         min_us: 500,            // Minimum pulse width (µs) for 0° (default)
//!         max_us: 2500,           // Maximum pulse width (µs) for max_degrees (default)
//!         max_degrees: 180,       // Maximum servo angle (degrees) (default)
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
//!     let servo_sweep = ServoSweep::new(p.PIN_12, p.PWM_SLICE6, spawner)?;
//!
//!     // Combine 40 animation steps into one array.
//!     const STEPS: [(u16, Duration); 40] = combine!(
//!         linear::<19>(0, 180, Duration::from_secs(2)), // 19 steps from 0° to 180°
//!         [(180, Duration::from_millis(400))],          // Hold at 180° for 400 ms
//!         linear::<19>(180, 0, Duration::from_secs(2)), // 19 steps from 180° to 0°
//!         [(0, Duration::from_millis(400))]             // Hold at 0° for 400 ms
//!     );
//!
//!     servo_sweep.animate(STEPS, AtEnd::Loop); // Loop the sweep animation
//!
//!     // Let it run in the background for 10 seconds, then relax.
//!     embassy_time::Timer::after(Duration::from_secs(10)).await;
//!     servo_sweep.relax();
//!
//!     pending().await // run forever
//! }
//! ```

// ============================================================================
// Submodules
// ============================================================================

pub mod servo_player_generated;
/// Combine multiple animation step arrays into one larger array.
///
/// This macro allows combining any number of const arrays with a clean syntax.
///
/// **Syntax:**
///
/// ```text
/// combine!()
/// combine!(<steps_expr>)
/// combine!(<first_steps_expr>, <second_steps_expr>, ... )
/// ```
///
/// See the [servo module documentation](mod@crate::servo) for usage.
#[doc(hidden)]
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
        $crate::servo::combine::<{FIRST.len()}, {SECOND.len()}, {FIRST.len() + SECOND.len()}>($first, $second)
    }};
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        const FIRST: &[(u16, ::embassy_time::Duration)] = &$first;
        const REST: &[(u16, ::embassy_time::Duration)] = &$crate::combine!($($rest),+);
        $crate::servo::combine::<{FIRST.len()}, {REST.len()}, {FIRST.len() + REST.len()}>($first, $crate::combine!($($rest),+))
    }};
}

/// Macro to generate a servo player struct type (includes syntax details).
///
/// This page provides the primary documentation for configuring individual servo players.
///
/// See the [servo module documentation](mod@crate::servo) for complete
/// examples.

///
/// **After reading the configuration details below, see also:**
///
/// - [`servo`](mod@crate::servo) module — Complete examples and usage
///   patterns
///
/// Use this macro when your project has a servo that needs scripted animation control.
/// The macro generates a struct type and spawns a background
/// task to execute animation sequences.
///
/// **Syntax:**
///
/// ```text
/// servo_player! {
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///         min_us: <u16_expr>,         // optional
///         max_us: <u16_expr>,         // optional
///         max_degrees: <u16_expr>,    // optional
///         max_steps: <usize_expr>,    // optional
///     }
/// }
/// ```
///
/// # Configuration
///
/// **Required fields:**
///
/// - `pin` — GPIO pin for servo
///
/// **Optional fields:**
///
/// - `min_us` — Minimum pulse width in microseconds for 0° (default: 500)
/// - `max_us` — Maximum pulse width in microseconds for max_degrees
///   (default: 2500)
/// - `max_degrees` — Maximum servo angle in degrees (default: 180)
/// - `max_steps` — Maximum number of animation steps (default: 16)
///
/// `max_steps = 0` disables animation and allocates no step storage; `set_degrees()`,
/// `hold()`, and `relax()` are still supported.

#[cfg(not(feature = "host"))]
#[doc(hidden)]
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
            vis: pub(self),
            name: $name,
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::ServoRp::DEFAULT_MAX_DEGREES,
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
            max_degrees: $crate::servo::ServoRp::DEFAULT_MAX_DEGREES,
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
        $crate::servo::paste::paste! {
            static [<$name:upper _SERVO_PLAYER_STATIC>]: $crate::servo::ServoPlayerStatic<$max_steps> =
                $crate::servo::ServoPlayerHandle::<$max_steps>::new_static();
            static [<$name:upper _SERVO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            #[allow(missing_docs)]
            $vis struct $name {
                servo_player_handle: $crate::servo::ServoPlayerHandle<$max_steps>,
            }

            #[allow(missing_docs)]
            impl $name {
                pub const MAX_STEPS: usize = $max_steps;

                /// Create the servo player and spawn its background task.
                ///
                /// The slice is automatically determined from the pin via the type
                /// system.
                ///
                /// # PWM Slice Calculation
                ///
                /// Calculate which [PWM slice](crate#glossary) a pin uses:
                /// `slice = (pin / 2) % 8`. For example, PIN_11 uses PWM_SLICE5
                /// ((11 / 2) % 8 = 5).
                ///
                /// # Parameters
                ///
                /// - `pin` — GPIO pin for servo
                /// - `slice` — PWM slice corresponding to the pin
                /// - `spawner` — Task spawner for background operations
                ///
                /// See the `ServoPlayer` struct example for usage.
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
                    let servo_player_handle =
                        $crate::servo::ServoPlayerHandle::new(&[<$name:upper _SERVO_PLAYER_STATIC>]);
                    Ok([<$name:upper _SERVO_PLAYER_CELL>].init(Self { servo_player_handle }))
                }
            }

            impl $crate::servo::Servo for $name {
                const DEFAULT_MAX_DEGREES: u16 = $max_degrees;

                fn set_degrees(&self, degrees: u16) {
                    $crate::servo::__servo_player_set_degrees(&self.servo_player_handle, degrees);
                }

                fn hold(&self) {
                    $crate::servo::__servo_player_hold(&self.servo_player_handle);
                }

                fn relax(&self) {
                    $crate::servo::__servo_player_relax(&self.servo_player_handle);
                }
            }

            impl $crate::servo::ServoPlayer<$max_steps> for $name {
                const MAX_STEPS: usize = Self::MAX_STEPS;

                fn animate<I>(&self, steps: I, at_end: $crate::servo::AtEnd)
                where
                    I: ::core::iter::IntoIterator,
                    I::Item: ::core::borrow::Borrow<(u16, ::embassy_time::Duration)>,
                {
                    $crate::servo::__servo_player_animate(&self.servo_player_handle, steps, at_end);
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo::ServoPlayerStatic<$max_steps>,
                servo: $crate::servo::ServoRp<'static>,
            ) -> ! {
                $crate::servo::device_loop(servo_player_static, servo).await
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
        $crate::servo::paste::paste! {
            static [<$name:upper _SERVO_PLAYER_STATIC>]: $crate::servo::ServoPlayerStatic<$max_steps> =
                $crate::servo::ServoPlayerHandle::<$max_steps>::new_static();
            static [<$name:upper _SERVO_PLAYER_CELL>]: ::static_cell::StaticCell<$name> =
                ::static_cell::StaticCell::new();

            #[allow(missing_docs)]
            $vis struct $name {
                servo_player_handle: $crate::servo::ServoPlayerHandle<$max_steps>,
            }

            #[allow(missing_docs)]
            impl $name {
                pub const MAX_STEPS: usize = $max_steps;

                /// Create the servo player and spawn its background task.
                ///
                /// # PWM Slice Calculation
                ///
                /// Calculate which [PWM slice](crate#glossary) a pin uses:
                /// `slice = (pin / 2) % 8`. For example, PIN_11 uses PWM_SLICE5
                /// ((11 / 2) % 8 = 5).
                ///
                /// # Parameters
                ///
                /// - `pin` — GPIO pin for servo
                /// - `slice` — PWM slice corresponding to the pin
                /// - `spawner` — Task spawner for background operations
                ///
                /// See the `ServoPlayer` struct example for usage.
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
                    let servo_player_handle =
                        $crate::servo::ServoPlayerHandle::new(&[<$name:upper _SERVO_PLAYER_STATIC>]);
                    Ok([<$name:upper _SERVO_PLAYER_CELL>].init(Self { servo_player_handle }))
                }
            }

            impl $crate::servo::Servo for $name {
                const DEFAULT_MAX_DEGREES: u16 = $max_degrees;

                fn set_degrees(&self, degrees: u16) {
                    $crate::servo::__servo_player_set_degrees(&self.servo_player_handle, degrees);
                }

                fn hold(&self) {
                    $crate::servo::__servo_player_hold(&self.servo_player_handle);
                }

                fn relax(&self) {
                    $crate::servo::__servo_player_relax(&self.servo_player_handle);
                }
            }

            impl $crate::servo::ServoPlayer<$max_steps> for $name {
                const MAX_STEPS: usize = Self::MAX_STEPS;

                fn animate<I>(&self, steps: I, at_end: $crate::servo::AtEnd)
                where
                    I: ::core::iter::IntoIterator,
                    I::Item: ::core::borrow::Borrow<(u16, ::embassy_time::Duration)>,
                {
                    $crate::servo::__servo_player_animate(&self.servo_player_handle, steps, at_end);
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _servo_player_task>](
                servo_player_static: &'static $crate::servo::ServoPlayerStatic<$max_steps>,
                servo: $crate::servo::ServoRp<'static>,
            ) -> ! {
                $crate::servo::device_loop(servo_player_static, servo).await
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
        $crate::servo::ServoRp::new_output_a(
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
        $crate::servo::ServoRp::new_output_b(
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
            vis: pub(self),
            name: ServoPlayerGenerated,
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::ServoRp::DEFAULT_MAX_DEGREES,
            max_steps: 16,
            fields: [ $($fields)* ]
        }
    };
}
