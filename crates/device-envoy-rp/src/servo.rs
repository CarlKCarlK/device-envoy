//! A device abstraction for hobby servos.
//!
//! This module provides both direct servo control ([`servo!`](macro@crate::servo::servo)) and
//! servo animation ([`servo_player!`](macro@crate::servo::servo_player)).
//!
//! Use the [`servo!`] macro for a keyword-driven constructor with defaults.
//!
//! **After reading the examples below, see also:**
//!
//! - [`servo!`](macro@crate::servo::servo) — Direct servo control without animation support.
//! - [`servo_player!`](macro@crate::servo::servo_player) — Macro to generate a servo player struct
//!   type (includes syntax details). See [`ServoPlayerGenerated`](servo_player_generated::ServoPlayerGenerated)
//!   for a sample of a generated type.
//! - [`combine!`](macro@crate::servo::combine) & [`linear`] — Macro and function for creating
//!   complex motion sequences.
//! - [`Servo`] — Trait defining core methods and constants for direct servo control.
//! - [`ServoPlayer`] — Trait defining animation methods for servos and extending [`Servo`]
//!   (`set_degrees`, `hold`, and `relax`). Depending on method-resolution context,
//!   call sites may still need [`Servo`] in scope.
//!
#![doc = include_str!("../docs/how_servos_work.md")]
//!
//! This device abstraction, `servo_player`, adds a background software task around the hardware
//! control signal.
//!
//! # Controlling Multiple Servos
//!
//! Supports up to eight servos, one per [PWM slice](crate#glossary) resource. To calculate which PWM slice a pin uses,
//! use the formula: `PWM slice = (pin / 2) % 8`. For example, PIN_10 and PIN_11 must both use PWM_SLICE5
//! ((10 / 2) % 8 = 5, (11 / 2) % 8 = 5). Therefore, either of these two pins can have a servo, but not both.
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
//! # use defmt_rtt as _;
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
//! This example combines 40 animation steps using `linear` and `combine!` to
//! sweep up, hold, sweep down, hold pattern. Here, the generated struct type is named
//! `ServoSweep`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use defmt_rtt as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy_rp::{Result, servo::{AtEnd, Servo as _, ServoPlayer as _, combine, linear, servo_player}};
//! use embassy_time::Duration;
//!
//! servo_player! {
//!     ServoSweep {
//!         pin: PIN_12,
//!         max_steps: 40,
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
//!     const STEPS: [(u16, Duration); 40] = combine!(
//!         linear::<19>(0, 180, Duration::from_secs(2)),
//!         [(180, Duration::from_millis(400))],
//!         linear::<19>(180, 0, Duration::from_secs(2)),
//!         [(0, Duration::from_millis(400))]
//!     );
//!
//!     servo_sweep.animate(STEPS, AtEnd::Loop);
//!     pending().await
//! }
//! ```
use core::cell::{Cell, RefCell};
use defmt::info;
pub use device_envoy_core::servo::Servo;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pwm::{Config, Pwm};

#[doc(inline)]
pub use crate::combine;
#[doc(inline)]
pub use crate::servo_player::servo_player;
pub use device_envoy_core::servo::{
    AtEnd, ServoPlayer, ServoPlayerHandle, ServoPlayerStatic, combine, linear,
};

/// Sample generated servo-player type documentation.
pub mod servo_player_generated {
    #[cfg(doc)]
    pub use crate::servo_player::servo_player_generated::*;
}
#[doc(hidden)]
pub use device_envoy_core::servo::{
    __servo_player_animate, __servo_player_hold, __servo_player_relax, __servo_player_set_degrees,
    device_loop,
};
#[doc(hidden)]
pub use paste;

const SERVO_PERIOD_US: u16 = 20_000; // 20 ms

/// Default minimum pulse width for hobby servos (microseconds).
pub const SERVO_MIN_US_DEFAULT: u16 = 500;

/// Default maximum pulse width for hobby servos (microseconds).
pub const SERVO_MAX_US_DEFAULT: u16 = 2_500;

/// Create a servo with keyword arguments and default pulse widths.
///
/// **Syntax:**
///
/// ```text
/// servo! {
///     pin: <pin_expr>,
///     slice: <pwm_slice_expr>,
///     channel: A | B,             // optional
///     odd: <bool_expr>,           // optional
///     even: <bool_expr>,          // optional
///     min_us: <u16_expr>,         // optional
///     max_us: <u16_expr>,         // optional
///     max_degrees: <u16_expr>,    // optional
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` - GPIO pin for servo output
/// - `slice` - [PWM slice](crate#glossary) resource
///
/// **Optional fields:**
///
/// - `channel: A | B` - Explicitly choose PWM output channel
/// - `odd` / `even` - Shorthand channel selection (`odd` => `B`, `even` => `A`)
/// - `min_us` - Minimum pulse width in microseconds for 0° (default: [`SERVO_MIN_US_DEFAULT`])
/// - `max_us` - Maximum pulse width in microseconds for `max_degrees` (default: [`SERVO_MAX_US_DEFAULT`])
/// - `max_degrees` - Maximum servo angle in degrees (default: `180`)
///
/// See the [servo module documentation](mod@crate::servo) for details and examples.
#[macro_export]
#[doc(hidden)]
macro_rules! servo {
    ($($tt:tt)*) => { $crate::__servo_impl! { $($tt)* } };
}
#[doc(inline)]
pub use servo;

// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_impl {
    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ ]
    ) => {
        $crate::__servo_impl! {
            @__build
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ pin: $pin_value:expr, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin_value,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ pin: $pin_value:expr ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin_value,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ slice: $slice_value:expr, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice_value,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ slice: $slice_value:expr ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice_value,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ min_us: $min_us_value:expr, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us_value,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ min_us: $min_us_value:expr ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us_value,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ max_us: $max_us_value:expr, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us_value,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ max_us: $max_us_value:expr ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us_value,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ max_degrees: $max_degrees_value:expr, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees_value,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ max_degrees: $max_degrees_value:expr ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: $channel,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees_value,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ channel: A, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ channel: A ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ channel: B, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ channel: B ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ even, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ even ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: A,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ odd, $($rest:tt)* ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ $($rest)* ]
        }
    };

    (@__fill_defaults
        pin: $pin:tt,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr,
        fields: [ odd ]
    ) => {
        $crate::__servo_impl! {
            @__fill_defaults
            pin: $pin,
            slice: $slice,
            channel: B,
            min_us: $min_us,
            max_us: $max_us,
            max_degrees: $max_degrees,
            fields: [ ]
        }
    };

    (@__build
        pin: _UNSET_,
        slice: $slice:tt,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
    ) => {
        compile_error!("servo! requires `pin: ...`");
    };

    (@__build
        pin: $pin:expr,
        slice: _UNSET_,
        channel: $channel:tt,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
    ) => {
        compile_error!("servo! requires `slice: ...`");
    };

    (@__build
        pin: $pin:expr,
        slice: $slice:expr,
        channel: _UNSET_,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
    ) => {
        $crate::servo::servo_from_pin_slice($pin, $slice, $min_us, $max_us, $max_degrees)
    };

    (@__build
        pin: $pin:expr,
        slice: $slice:expr,
        channel: A,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
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

    (@__build
        pin: $pin:expr,
        slice: $slice:expr,
        channel: B,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
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
        {
            $crate::__servo_validate_fields! { fields: [ $($fields)* ] }
            $crate::__servo_impl! {
                @__fill_defaults
                pin: _UNSET_,
                slice: _UNSET_,
                channel: _UNSET_,
                min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
                max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
                max_degrees: $crate::servo::ServoRp::DEFAULT_MAX_DEGREES,
                fields: [ $($fields)* ]
            }
        }
    };
}

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_validate_fields {
    (fields: [ $($fields:tt)* ]) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [],
            slice: [],
            channel: [],
            min_us: [],
            max_us: [],
            max_degrees: [],
            fields: [ $($fields)* ]
        }
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ ]
    ) => {};

    (@__parse
        pin: [],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ pin: $pin:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [set],
            slice: [$($slice_seen)?],
            channel: [$($channel_seen)?],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [set],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ pin: $pin:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `pin` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ slice: $slice:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [set],
            channel: [$($channel_seen)?],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [set],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ slice: $slice:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `slice` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ channel: A $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [set],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ channel: B $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [set],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [set],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ channel: A $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `channel` field");
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [set],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ channel: B $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `channel` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ even $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [set],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [set],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ even $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate channel selector (`channel`, `odd`, or `even`)");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ odd $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [set],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [set],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ odd $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate channel selector (`channel`, `odd`, or `even`)");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ min_us: $min_us:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [$($channel_seen)?],
            min_us: [set],
            max_us: [$($max_us_seen)?],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [set],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ min_us: $min_us:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `min_us` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ max_us: $max_us:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [$($channel_seen)?],
            min_us: [$($min_us_seen)?],
            max_us: [set],
            max_degrees: [$($max_degrees_seen)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [set],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ max_us: $max_us:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `max_us` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [],
        fields: [ max_degrees: $max_degrees:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_validate_fields! {
            @__parse
            pin: [$($pin_seen)?],
            slice: [$($slice_seen)?],
            channel: [$($channel_seen)?],
            min_us: [$($min_us_seen)?],
            max_us: [$($max_us_seen)?],
            max_degrees: [set],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [set],
        fields: [ max_degrees: $max_degrees:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `max_degrees` field");
    };

    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ $field:ident : $($value:tt)+ ]
    ) => {
        compile_error!(
            "servo! unknown field; expected `pin`, `slice`, `channel`, `odd`, `even`, `min_us`, `max_us`, or `max_degrees`"
        );
    };
    (@__parse
        pin: [$($pin_seen:tt)?],
        slice: [$($slice_seen:tt)?],
        channel: [$($channel_seen:tt)?],
        min_us: [$($min_us_seen:tt)?],
        max_us: [$($max_us_seen:tt)?],
        max_degrees: [$($max_degrees_seen:tt)?],
        fields: [ $unknown:tt $(, $($rest:tt)*)? ]
    ) => {
        compile_error!(
            "servo! unknown field; expected `pin`, `slice`, `channel`, `odd`, `even`, `min_us`, `max_us`, or `max_degrees`"
        );
    };
}

// Public for macro expansion in downstream crates.
#[doc(hidden)]
pub trait ServoPwmPin<S: embassy_rp::PeripheralType>: embassy_rp::PeripheralType {
    const IS_CHANNEL_A: bool;
    fn new_pwm<'d>(slice: embassy_rp::Peri<'d, S>, pin: embassy_rp::Peri<'d, Self>) -> Pwm<'d>;
}

// Public for macro expansion in downstream crates.
#[doc(hidden)]
pub fn servo_from_pin_slice<'d, P, S>(
    pin: embassy_rp::Peri<'d, P>,
    slice: embassy_rp::Peri<'d, S>,
    min_us: u16,
    max_us: u16,
    max_degrees: u16,
) -> ServoRp<'d>
where
    P: ServoPwmPin<S>,
    S: embassy_rp::PeripheralType,
{
    let pwm = P::new_pwm(slice, pin);
    if P::IS_CHANNEL_A {
        ServoRp::new_output_a(pwm, min_us, max_us, max_degrees)
    } else {
        ServoRp::new_output_b(pwm, min_us, max_us, max_degrees)
    }
}

macro_rules! servo_pin_map {
    ($pin:ident, $slice:ident, A) => {
        impl ServoPwmPin<embassy_rp::peripherals::$slice> for embassy_rp::peripherals::$pin {
            const IS_CHANNEL_A: bool = true;
            fn new_pwm<'d>(
                slice: embassy_rp::Peri<'d, embassy_rp::peripherals::$slice>,
                pin: embassy_rp::Peri<'d, Self>,
            ) -> Pwm<'d> {
                embassy_rp::pwm::Pwm::new_output_a(slice, pin, Config::default())
            }
        }
    };
    ($pin:ident, $slice:ident, B) => {
        impl ServoPwmPin<embassy_rp::peripherals::$slice> for embassy_rp::peripherals::$pin {
            const IS_CHANNEL_A: bool = false;
            fn new_pwm<'d>(
                slice: embassy_rp::Peri<'d, embassy_rp::peripherals::$slice>,
                pin: embassy_rp::Peri<'d, Self>,
            ) -> Pwm<'d> {
                embassy_rp::pwm::Pwm::new_output_b(slice, pin, Config::default())
            }
        }
    };
}

servo_pin_map!(PIN_0, PWM_SLICE0, A);
servo_pin_map!(PIN_1, PWM_SLICE0, B);
servo_pin_map!(PIN_2, PWM_SLICE1, A);
servo_pin_map!(PIN_3, PWM_SLICE1, B);
servo_pin_map!(PIN_4, PWM_SLICE2, A);
servo_pin_map!(PIN_5, PWM_SLICE2, B);
servo_pin_map!(PIN_6, PWM_SLICE3, A);
servo_pin_map!(PIN_7, PWM_SLICE3, B);
servo_pin_map!(PIN_8, PWM_SLICE4, A);
servo_pin_map!(PIN_9, PWM_SLICE4, B);
servo_pin_map!(PIN_10, PWM_SLICE5, A);
servo_pin_map!(PIN_11, PWM_SLICE5, B);
servo_pin_map!(PIN_12, PWM_SLICE6, A);
servo_pin_map!(PIN_13, PWM_SLICE6, B);
servo_pin_map!(PIN_14, PWM_SLICE7, A);
servo_pin_map!(PIN_15, PWM_SLICE7, B);
servo_pin_map!(PIN_16, PWM_SLICE0, A);
servo_pin_map!(PIN_17, PWM_SLICE0, B);
servo_pin_map!(PIN_18, PWM_SLICE1, A);
servo_pin_map!(PIN_19, PWM_SLICE1, B);
servo_pin_map!(PIN_20, PWM_SLICE2, A);
servo_pin_map!(PIN_21, PWM_SLICE2, B);
servo_pin_map!(PIN_22, PWM_SLICE3, A);
servo_pin_map!(PIN_23, PWM_SLICE3, B);
servo_pin_map!(PIN_24, PWM_SLICE4, A);
servo_pin_map!(PIN_25, PWM_SLICE4, B);
servo_pin_map!(PIN_26, PWM_SLICE5, A);
servo_pin_map!(PIN_27, PWM_SLICE5, B);
servo_pin_map!(PIN_28, PWM_SLICE6, A);
servo_pin_map!(PIN_29, PWM_SLICE6, B);

#[cfg(feature = "pico2")]
servo_pin_map!(PIN_30, PWM_SLICE7, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_31, PWM_SLICE7, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_32, PWM_SLICE8, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_33, PWM_SLICE8, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_34, PWM_SLICE9, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_35, PWM_SLICE9, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_36, PWM_SLICE10, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_37, PWM_SLICE10, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_38, PWM_SLICE11, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_39, PWM_SLICE11, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_40, PWM_SLICE8, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_41, PWM_SLICE8, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_42, PWM_SLICE9, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_43, PWM_SLICE9, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_44, PWM_SLICE10, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_45, PWM_SLICE10, B);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_46, PWM_SLICE11, A);
#[cfg(feature = "pico2")]
servo_pin_map!(PIN_47, PWM_SLICE11, B);

/// A device abstraction for hobby servos.
///
/// Use [`servo!`](macro@crate::servo::servo) for direct, immediate control when you want to manually manage servo
/// positioning. Use [`servo_player`](mod@crate::servo) instead when you need
/// background animation sequences or want motion to continue while your code does other work.
///
#[doc = include_str!("../docs/how_servos_work.md")]
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::{servo, servo::Servo as _};
/// use embassy_time::{Duration, Timer};
/// # use core::panic::PanicInfo;
/// # #[panic_handler]
/// # fn panic(_info: &PanicInfo) -> ! { loop {} }
/// async fn example(p: embassy_rp::Peripherals) {
///     // Create a servo on GPIO 11.
///     // GPIO 11 → (11/2) % 8 = 5 → PWM_SLICE5
///     let mut servo = servo! {
///         pin: p.PIN_11,
///         slice: p.PWM_SLICE5,
///     };
///
///     servo.set_degrees(45);                          // Move to 45 degrees and hold.
///     Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
///     servo.set_degrees(90);                          // Move to 90 degrees and hold.
///     Timer::after(Duration::from_secs(1)).await;     // Give servo reasonable time to reach position
///     servo.relax();                                  // Let the servo relax. It will re-enable on next set_degrees()
/// }
/// ```
pub struct ServoRp<'d> {
    pwm: RefCell<Pwm<'d>>,
    cfg: RefCell<Config>, // Store config to avoid recreating default (which resets divider)
    top: u16,
    min_us: u16,
    max_us: u16,
    max_degrees: u16,
    channel: ServoChannel, // Track which channel (A or B) this servo uses
    state: Cell<ServoState>,
}

#[derive(Debug, Clone, Copy)]
enum ServoChannel {
    A,
    B,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ServoState {
    Disabled,
    Enabled,
}

impl<'d> ServoRp<'d> {
    /// Default maximum rotation range in degrees (180°).
    pub const DEFAULT_MAX_DEGREES: u16 = <Self as Servo>::DEFAULT_MAX_DEGREES;

    /// Create a servo on a PWM output A channel.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    pub(crate) fn new_output_a(pwm: Pwm<'d>, min_us: u16, max_us: u16, max_degrees: u16) -> Self {
        Self::init(pwm, ServoChannel::A, min_us, max_us, max_degrees)
    }

    /// Create a servo on a PWM output B channel.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    pub(crate) fn new_output_b(pwm: Pwm<'d>, min_us: u16, max_us: u16, max_degrees: u16) -> Self {
        Self::init(pwm, ServoChannel::B, min_us, max_us, max_degrees)
    }

    /// Configure PWM and initialize servo. Internal shared logic.
    fn init(
        mut pwm: Pwm<'d>,
        channel: ServoChannel,
        min_us: u16,
        max_us: u16,
        max_degrees: u16,
    ) -> Self {
        // TODO consider if these could/should be checked at compile time.
        assert!(min_us < max_us, "min_us must be less than max_us");
        assert!(max_degrees > 0, "max_degrees must be positive");
        let clk = clk_sys_freq() as u64; // Hz
        // Aim for tick ≈ 1 µs: divider = clk_sys / 1_000_000 (with /16 fractional)
        let mut div_int = (clk / 1_000_000).clamp(1, 255) as u16;
        let rem = clk.saturating_sub(div_int as u64 * 1_000_000);
        let mut div_frac = ((rem * 16 + 500_000) / 1_000_000).clamp(0, 15) as u8;
        if div_frac == 16 {
            div_frac = 0;
            div_int = (div_int + 1).min(255);
        }

        let top = SERVO_PERIOD_US - 1; // 19999 -> 20_000 ticks/frame
        assert!(min_us <= top, "min_us must fit in the PWM frame");
        assert!(max_us <= top, "max_us must fit in the PWM frame");

        let mut cfg = Config::default();
        cfg.top = top;
        cfg.phase_correct = false; // edge-aligned => exact 1 µs steps
        // Apply divider: use the integer part as u8 which has a From impl
        cfg.divider = (div_int as u8).into();

        // Set the appropriate compare register based on channel
        match channel {
            ServoChannel::A => cfg.compare_a = 1500, // start ~center
            ServoChannel::B => cfg.compare_b = 1500, // start ~center
        }

        cfg.enable = true; // Enable PWM output
        pwm.set_config(&cfg);

        info!(
            "servo clk={}Hz div={}.{} top={}",
            clk, div_int, div_frac, top
        );

        let servo = Self {
            pwm: RefCell::new(pwm),
            cfg: RefCell::new(cfg), // Store config to avoid losing divider on reconfiguration
            top,
            min_us,
            max_us,
            max_degrees,
            channel,
            state: Cell::new(ServoState::Enabled),
        };
        let center_us = min_us + (max_us - min_us) / 2;
        servo.set_pulse_us(center_us);
        servo
    }

    /// Set raw pulse width in microseconds.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    /// NOTE: only update the *compare* register; do not reconfigure the slice.
    #[doc(hidden)]
    pub fn set_pulse_us(&self, us: u16) {
        assert!(us <= self.top, "pulse width must fit in the PWM frame");
        // One tick ≈ 1 µs, so compare = us.
        // CRITICAL: Update our stored config and reapply it WITH the divider intact.
        // This prevents the divider from being reset to default.
        let mut cfg = self.cfg.borrow_mut();
        match self.channel {
            ServoChannel::A => cfg.compare_a = us,
            ServoChannel::B => cfg.compare_b = us,
        }
        self.pwm.borrow_mut().set_config(&cfg);
    }

    fn ensure_enabled(&self) {
        if self.state.get() == ServoState::Enabled {
            return;
        }

        let mut cfg = self.cfg.borrow_mut();
        cfg.enable = true;
        self.pwm.borrow_mut().set_config(&cfg);
        self.state.set(ServoState::Enabled);
    }
}

impl<'d> Servo for ServoRp<'d> {
    const DEFAULT_MAX_DEGREES: u16 = 180;

    /// Set position in degrees 0..=max_degrees mapped into [min_us, max_us].
    ///
    /// Automatically enables the servo if it was disabled.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    fn set_degrees(&self, degrees: u16) {
        assert!((0..=self.max_degrees).contains(&degrees));
        self.ensure_enabled();
        let us = self.min_us as u32
            + (u32::from(degrees)) * (u32::from(self.max_us) - u32::from(self.min_us))
                / u32::from(self.max_degrees);
        info!("Servo set_degrees({}) -> {}µs", degrees, us);
        self.set_pulse_us(us as u16);
    }

    /// Stop sending control signals to the servo.
    ///
    /// This allows the servo to relax and move freely, reducing power consumption
    /// and mechanical stress.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    fn relax(&self) {
        if self.state.get() == ServoState::Disabled {
            return;
        }

        let mut cfg = self.cfg.borrow_mut();
        cfg.enable = false;
        self.pwm.borrow_mut().set_config(&cfg);
        self.state.set(ServoState::Disabled);
    }

    /// Resume sending control signals to the servo.
    ///
    /// The servo will move back to its last commanded position.
    fn hold(&self) {
        if self.state.get() == ServoState::Enabled {
            return;
        }

        let mut cfg = self.cfg.borrow_mut();
        cfg.enable = true;
        self.pwm.borrow_mut().set_config(&cfg);
        self.state.set(ServoState::Enabled);
    }
}
