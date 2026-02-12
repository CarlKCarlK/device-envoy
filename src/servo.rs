//! A device abstraction for hobby servos.
//!
//! This module provides a simple interface for controlling hobby positional servo motors
//! like the SG90. See [`Servo`] for usage examples.
//!
//! Use the [`servo!`] macro for a keyword-driven constructor with defaults.

use defmt::info;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pwm::{Config, Pwm};

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
/// Required fields: `pin`, `slice`.
///
/// Optional fields: `min_us`, `max_us`, `max_degrees` (defaults to
/// [`SERVO_MIN_US_DEFAULT`]/[`SERVO_MAX_US_DEFAULT`]/[`Servo::DEFAULT_MAX_DEGREES`]),
/// plus `channel: A/B` or `odd`/`even` to override the inferred channel.
///
/// See [`Servo`] for details and examples.
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

    (@__build
        pin: $pin:expr,
        slice: $slice:expr,
        channel: B,
        min_us: $min_us:expr,
        max_us: $max_us:expr,
        max_degrees: $max_degrees:expr
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
        $crate::__servo_impl! {
            @__fill_defaults
            pin: _UNSET_,
            slice: _UNSET_,
            channel: _UNSET_,
            min_us: $crate::servo::SERVO_MIN_US_DEFAULT,
            max_us: $crate::servo::SERVO_MAX_US_DEFAULT,
            max_degrees: $crate::servo::Servo::DEFAULT_MAX_DEGREES,
            fields: [ $($fields)* ]
        }
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
) -> Servo<'d>
where
    P: ServoPwmPin<S>,
    S: embassy_rp::PeripheralType,
{
    let pwm = P::new_pwm(slice, pin);
    if P::IS_CHANNEL_A {
        Servo::new_output_a(pwm, min_us, max_us, max_degrees)
    } else {
        Servo::new_output_b(pwm, min_us, max_us, max_degrees)
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
/// Use `Servo` for direct, immediate control when you want to manually manage servo
/// positioning. Use [`servo_player`](mod@crate::servo_player) instead when you need
/// background animation sequences or want motion to continue while your code does other work.
///
#[doc = include_str!("../docs/how_servos_work.md")]
///
/// # Examples
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy::{servo, servo::Servo};
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
pub struct Servo<'d> {
    pwm: Pwm<'d>,
    cfg: Config, // Store config to avoid recreating default (which resets divider)
    top: u16,
    min_us: u16,
    max_us: u16,
    max_degrees: u16,
    channel: ServoChannel, // Track which channel (A or B) this servo uses
    state: ServoState,
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

impl<'d> Servo<'d> {
    /// Default maximum rotation range in degrees (180°).
    pub const DEFAULT_MAX_DEGREES: u16 = 180;

    /// Create a servo on a PWM output A channel.
    ///
    /// See the [`Servo`] example for usage.
    pub(crate) fn new_output_a(pwm: Pwm<'d>, min_us: u16, max_us: u16, max_degrees: u16) -> Self {
        Self::init(pwm, ServoChannel::A, min_us, max_us, max_degrees)
    }

    /// Create a servo on a PWM output B channel.
    ///
    /// See the [`Servo`] example for usage.
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
        // TODO: consider if these could/should be checked at compile time.
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

        let mut servo = Self {
            pwm,
            cfg, // Store config to avoid losing divider on reconfiguration
            top,
            min_us,
            max_us,
            max_degrees,
            channel,
            state: ServoState::Enabled,
        };
        let center_us = min_us + (max_us - min_us) / 2;
        servo.set_pulse_us(center_us);
        servo
    }

    /// Set position in degrees 0..=max_degrees mapped into [min_us, max_us].
    ///
    /// Automatically enables the servo if it was disabled.
    ///
    /// See the [`Servo`] example for usage.
    pub fn set_degrees(&mut self, degrees: u16) {
        assert!((0..=self.max_degrees).contains(&degrees));
        self.ensure_enabled();
        let us = self.min_us as u32
            + (u32::from(degrees)) * (u32::from(self.max_us) - u32::from(self.min_us))
                / u32::from(self.max_degrees);
        info!("Servo set_degrees({}) -> {}µs", degrees, us);
        self.set_pulse_us(us as u16);
    }

    /// Set raw pulse width in microseconds.
    ///
    /// See the [`Servo`] example for usage.
    /// NOTE: only update the *compare* register; do not reconfigure the slice.
    #[doc(hidden)]
    pub fn set_pulse_us(&mut self, us: u16) {
        assert!(us <= self.top, "pulse width must fit in the PWM frame");
        // One tick ≈ 1 µs, so compare = us.
        // CRITICAL: Update our stored config and reapply it WITH the divider intact.
        // This prevents the divider from being reset to default.
        match self.channel {
            ServoChannel::A => self.cfg.compare_a = us,
            ServoChannel::B => self.cfg.compare_b = us,
        }
        self.pwm.set_config(&self.cfg);
    }

    fn ensure_enabled(&mut self) {
        if self.state == ServoState::Enabled {
            return;
        }

        self.cfg.enable = true;
        self.pwm.set_config(&self.cfg);
        self.state = ServoState::Enabled;
    }

    /// Stop sending control signals to the servo.
    ///
    /// This allows the servo to relax and move freely, reducing power consumption
    /// and mechanical stress.
    ///
    /// See the [`Servo`] example for usage.
    pub fn relax(&mut self) {
        if self.state == ServoState::Disabled {
            return;
        }

        self.cfg.enable = false;
        self.pwm.set_config(&self.cfg);
        self.state = ServoState::Disabled;
    }

    /// Resume sending control signals to the servo.
    ///
    /// The servo will move back to its last commanded position.
    pub fn hold(&mut self) {
        if self.state == ServoState::Enabled {
            return;
        }

        self.cfg.enable = true;
        self.pwm.set_config(&self.cfg);
        self.state = ServoState::Enabled;
    }
}
