//! A device abstraction for hobby servos on ESP LEDC PWM.
//!
//! This module provides both direct servo control ([`servo!`](macro@crate::servo::servo)) and
//! servo animationl ([`servo_player!`](macro@crate::servo::servo_player)).
//!
//! Use [`servo!`] for a keyword-driven typed constructor.
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
//! - [`ServoPlayer`] — Trait defining core methods and constants for animatable servos.
//!
#![doc = include_str!("../docs/how_servos_work.md")]
//!
//! This device abstraction, `servo_player`, adds a background software task around the hardware
//! control signal.
//!
//! # Controlling Multiple Servos
//!
//! Supports multiple servos, where each servo consumes one [LEDC](crate#glossary) timer resource and
//! one [LEDC](crate#glossary) channel resource. The macro-generated link-time ownership claims enforce
//! this exclusivity so duplicate timer/channel selections fail at link time.
//!
//! # Example: Basic Servo Control
//!
//! This example demonstrates basic servo control: moving to a position, relaxing,
//! and using animation. Here, the generated struct type is named `Servo11`.
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use esp_backtrace as _;
//! # use core::convert::Infallible;
//! use device_envoy_esp::{Result, init_and_start, servo::{Servo as _, servo}};
//! use embassy_time::{Duration, Timer};
//!
//! servo! {
//!     Servo11 {
//!         pin: GPIO11,
//!         timer: Timer0,
//!         channel: Channel0,
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(_spawner: embassy_executor::Spawner) -> ! {
//! #     let err = inner_main().await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn inner_main() -> Result<Infallible> {
//!     init_and_start!(p, ledc: ledc);
//!     let servo11 = Servo11::new(&ledc, p.GPIO11)?;
//!     servo11.set_degrees(45);
//!     Timer::after(Duration::from_secs(1)).await;
//!     servo11.relax();
//!     core::future::pending().await
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
//! # use esp_backtrace as _;
//! # use core::convert::Infallible;
//! use device_envoy_esp::{Result, init_and_start, servo::{AtEnd, Servo as _, ServoPlayer as _, combine, linear, servo_player}};
//! use embassy_time::Duration;
//!
//! servo_player! {
//!     ServoSweep {
//!         pin: GPIO12,
//!         timer: Timer1,
//!         channel: Channel1,
//!         max_steps: 40,
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = inner_main(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, ledc: ledc);
//!     let servo_sweep = ServoSweep::new(&ledc, p.GPIO12, spawner)?;
//!     const STEPS: [(u16, Duration); 40] = combine!(
//!         linear::<19>(0, 180, Duration::from_secs(2)),
//!         [(180, Duration::from_millis(400))],
//!         linear::<19>(180, 0, Duration::from_secs(2)),
//!         [(0, Duration::from_millis(400))]
//!     );
//!     servo_sweep.animate(STEPS, AtEnd::Loop);
//!     core::future::pending().await
//! }
//! ```

use crate::Result;
use core::cell::RefCell;
pub use device_envoy_core::servo::Servo;
use esp_hal::gpio::{interconnect::PeripheralOutput, DriveMode};
use esp_hal::ledc::{channel, timer, LowSpeed};
use esp_hal::ledc::{channel::ChannelHW, channel::ChannelIFace, timer::TimerIFace};
use esp_hal::time::Rate;
use static_cell::StaticCell;

#[doc(inline)]
pub use crate::combine;
#[doc(inline)]
pub use crate::servo_player::servo_player;
#[doc(hidden)]
pub use device_envoy_core::servo::{
    __servo_player_animate, __servo_player_hold, __servo_player_relax, __servo_player_set_degrees,
    device_loop,
};
pub use device_envoy_core::servo::{
    combine, linear, AtEnd, ServoPlayer, ServoPlayerHandle, ServoPlayerStatic,
};

/// Sample generated servo-player type documentation.
pub mod servo_player_generated {
    #[cfg(doc)]
    pub use crate::servo_player::servo_player_generated::*;
}

const SERVO_PERIOD_US: u32 = 20_000;
const SERVO_TIMER_DUTY_BITS: u32 = 14;

/// Default minimum pulse width for hobby servos (microseconds).
pub const SERVO_MIN_US_DEFAULT: u32 = 500;

/// Default maximum pulse width for hobby servos (microseconds).
pub const SERVO_MAX_US_DEFAULT: u32 = 2_500;

/// LEDC-backed servo static resources and motion configuration.
pub struct ServoStatic {
    timer: StaticCell<timer::Timer<'static, LowSpeed>>,
    channel: StaticCell<channel::Channel<'static, LowSpeed>>,
    timer_number: timer::Number,
    channel_number: channel::Number,
    min_us: u32,
    max_us: u32,
    max_degrees: u16,
}

impl ServoStatic {
    /// Create static resources for one servo output.
    #[must_use]
    pub const fn new_static(
        timer_number: timer::Number,
        channel_number: channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
    ) -> Self {
        assert!(min_us < max_us, "min_us must be less than max_us");
        assert!(max_degrees > 0, "max_degrees must be positive");
        Self {
            timer: StaticCell::new(),
            channel: StaticCell::new(),
            timer_number,
            channel_number,
            min_us,
            max_us,
            max_degrees,
        }
    }
}

/// A direct servo output using one LEDC timer and one LEDC channel.
///
/// This type is public only because `servo!` and `servo_player!` macro
/// expansions in downstream crates reference it.
#[doc(hidden)]
pub struct ServoEsp {
    channel: RefCell<&'static mut channel::Channel<'static, LowSpeed>>,
    min_us: u32,
    max_us: u32,
    max_degrees: u16,
}

impl ServoEsp {
    /// Default maximum rotation range in degrees.
    pub const DEFAULT_MAX_DEGREES: u16 = <Self as Servo>::DEFAULT_MAX_DEGREES;

    /// Create a servo from static resources and a GPIO output pin.
    pub fn new(
        servo_static: &'static ServoStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl PeripheralOutput<'static>,
    ) -> Result<Self> {
        let timer = servo_static
            .timer
            .init(ledc.timer::<LowSpeed>(servo_static.timer_number));
        timer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })?;

        let channel = servo_static
            .channel
            .init(ledc.channel(servo_static.channel_number, pin));
        channel.configure(channel::config::Config {
            timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })?;

        Ok(Self {
            channel: RefCell::new(channel),
            min_us: servo_static.min_us,
            max_us: servo_static.max_us,
            max_degrees: servo_static.max_degrees,
        })
    }

    fn pulse_for_degrees(&self, degrees: u16) -> u32 {
        let pulse_span = self.max_us - self.min_us;
        self.min_us
            + (u32::from(degrees) * pulse_span + u32::from(self.max_degrees / 2))
                / u32::from(self.max_degrees)
    }

    fn degrees_to_duty(&self, degrees: u16) -> u32 {
        let pulse_us = self.pulse_for_degrees(degrees);
        let duty_range = 1u32 << SERVO_TIMER_DUTY_BITS;
        ((pulse_us * duty_range) + (SERVO_PERIOD_US / 2)) / SERVO_PERIOD_US
    }
}

impl Servo for ServoEsp {
    const DEFAULT_MAX_DEGREES: u16 = 180;

    /// Set position in degrees `0..=max_degrees`.
    fn set_degrees(&self, degrees: u16) {
        assert!(degrees <= self.max_degrees);
        let duty = self.degrees_to_duty(degrees);
        self.channel.borrow_mut().set_duty_hw(duty);
    }

    /// Keep driving pulses at the last commanded angle.
    fn hold(&self) {}

    /// Stop driving pulses.
    fn relax(&self) {
        self.channel
            .borrow_mut()
            .set_duty(0)
            .expect("LEDC set_duty failed in Servo::relax");
    }
}

#[doc(hidden)]
pub use paste;

/// Macro to generate a direct-servo struct type (includes syntax details).
///
/// **See the [servo module documentation](mod@crate::servo) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// servo! {
///     <Name> {
///         pin: <pin_ident>,
///         timer: <timer_ident>,
///         channel: <channel_ident>,
///         min_us: <u32_expr>,         // optional
///         max_us: <u32_expr>,         // optional
///         max_degrees: <u16_expr>,    // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` - GPIO pin for servo output
/// - `timer` - [LEDC](crate#glossary) timer resource
/// - `channel` - [LEDC](crate#glossary) channel resource
///
/// **Optional fields:**
///
/// - `min_us` - Minimum pulse width in microseconds for 0° (default: `500`)
/// - `max_us` - Maximum pulse width in microseconds for `max_degrees` (default: `2500`)
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

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_impl {
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [],
            timer: [],
            channel: [],
            min_us: [],
            max_us: [],
            max_degrees: [],
            fields: [ $($fields)* ]
        }
    };

    (@__parse
        name: $name:ident,
        pin: [],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ pin: $pin:ident $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$pin],
            timer: [$($timer)?],
            channel: [$($channel)?],
            min_us: [$($min_us)?],
            max_us: [$($max_us)?],
            max_degrees: [$($max_degrees)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$_pin_seen:ident],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ pin: $pin:ident $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `pin` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ timer: $timer:ident $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$($pin)?],
            timer: [$timer],
            channel: [$($channel)?],
            min_us: [$($min_us)?],
            max_us: [$($max_us)?],
            max_degrees: [$($max_degrees)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$_timer_seen:ident],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ timer: $timer:ident $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `timer` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ channel: $channel:ident $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$($pin)?],
            timer: [$($timer)?],
            channel: [$channel],
            min_us: [$($min_us)?],
            max_us: [$($max_us)?],
            max_degrees: [$($max_degrees)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$_channel_seen:ident],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ channel: $channel:ident $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `channel` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ min_us: $min_us:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$($pin)?],
            timer: [$($timer)?],
            channel: [$($channel)?],
            min_us: [$min_us],
            max_us: [$($max_us)?],
            max_degrees: [$($max_degrees)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$_min_us_seen:expr],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ min_us: $min_us:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `min_us` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ max_us: $max_us:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$($pin)?],
            timer: [$($timer)?],
            channel: [$($channel)?],
            min_us: [$($min_us)?],
            max_us: [$max_us],
            max_degrees: [$($max_degrees)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$_max_us_seen:expr],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ max_us: $max_us:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `max_us` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [],
        fields: [ max_degrees: $max_degrees:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__servo_impl! {
            @__parse
            name: $name,
            pin: [$($pin)?],
            timer: [$($timer)?],
            channel: [$($channel)?],
            min_us: [$($min_us)?],
            max_us: [$($max_us)?],
            max_degrees: [$max_degrees],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$_max_degrees_seen:expr],
        fields: [ max_degrees: $max_degrees:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("servo! duplicate `max_degrees` field");
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ ]
    ) => {
        $crate::__servo_impl! {
            @__finish
            name: $name,
            pin: [$($pin)?],
            timer: [$($timer)?],
            channel: [$($channel)?],
            min_us: [$($min_us)?],
            max_us: [$($max_us)?],
            max_degrees: [$($max_degrees)?]
        }
    };

    (@__parse
        name: $name:ident,
        pin: [$($pin:ident)?],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?],
        fields: [ $field:ident : $($value:tt)+ ]
    ) => {
        compile_error!(
            "servo! unknown field; expected `pin`, `timer`, `channel`, `min_us`, `max_us`, or `max_degrees`"
        );
    };

    (@__finish
        name: $name:ident,
        pin: [],
        timer: [$($timer:ident)?],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?]
    ) => {
        compile_error!("servo! missing required `pin` field");
    };
    (@__finish
        name: $name:ident,
        pin: [$pin:ident],
        timer: [],
        channel: [$($channel:ident)?],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?]
    ) => {
        compile_error!("servo! missing required `timer` field");
    };
    (@__finish
        name: $name:ident,
        pin: [$pin:ident],
        timer: [$timer:ident],
        channel: [],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?]
    ) => {
        compile_error!("servo! missing required `channel` field");
    };
    (@__finish
        name: $name:ident,
        pin: [$pin:ident],
        timer: [$timer:ident],
        channel: [$channel:ident],
        min_us: [$($min_us:expr)?],
        max_us: [$($max_us:expr)?],
        max_degrees: [$($max_degrees:expr)?]
    ) => {
        $crate::servo::paste::paste! {
            pub struct $name;

            // Link-time ownership claims: duplicate timer or channel selection across the
            // final binary should fail the link with duplicate symbol errors.
            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_timer_claim_ $timer:lower>]: u8 = 0;

            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_channel_claim_ $channel:lower>]: u8 = 0;

            static [<$name:upper _SERVO_STATIC>]: [<$name Static>] = $name::new_static();

            pub struct [<$name Static>] {
                servo_static: $crate::servo::ServoStatic,
            }

            impl $name {
                #[must_use]
                pub const fn new_static() -> [<$name Static>] {
                    [<$name Static>] {
                        servo_static: $crate::servo::ServoStatic::new_static(
                            ::esp_hal::ledc::timer::Number::$timer,
                            ::esp_hal::ledc::channel::Number::$channel,
                            $crate::__servo_impl!(@min_us $($min_us)?),
                            $crate::__servo_impl!(@max_us $($max_us)?),
                            $crate::__servo_impl!(@max_degrees $($max_degrees)?),
                        ),
                    }
                }

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: ::esp_hal::peripherals::$pin<'static>,
                ) -> $crate::Result<$crate::servo::ServoEsp> {
                    $crate::servo::ServoEsp::new(&[<$name:upper _SERVO_STATIC>].servo_static, ledc, pin)
                }
            }
        }
    };

    (@min_us $min_us:expr) => { $min_us };
    (@min_us) => { $crate::servo::SERVO_MIN_US_DEFAULT };
    (@max_us $max_us:expr) => { $max_us };
    (@max_us) => { $crate::servo::SERVO_MAX_US_DEFAULT };
    (@max_degrees $max_degrees:expr) => { $max_degrees };
    (@max_degrees) => { $crate::servo::ServoEsp::DEFAULT_MAX_DEGREES };
}
