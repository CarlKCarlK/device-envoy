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
//!   See [`ServoGenerated`](servo_generated::ServoGenerated) for a sample of a generated type.
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
pub use device_envoy_core::servo::{Direction, Servo};
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pwm::{Config, Pwm};

#[doc(inline)]
pub use crate::servo_player::servo_player;
pub use device_envoy_core::servo::{
    AtEnd, ServoPlayer, ServoPlayerHandle, ServoPlayerStatic, combine, linear,
};

pub mod servo_generated;

/// Module containing [`ServoPlayerGenerated`](servo_player_generated::ServoPlayerGenerated), the sample
/// struct type generated by the [`servo_player!`](macro@crate::servo::servo_player) macro.
pub mod servo_player_generated {
    pub use crate::servo_player::servo_player_generated::*;
}
#[doc(hidden)]
pub use device_envoy_core::servo::{
    __servo_player_animate, __servo_player_hold, __servo_player_relax, __servo_player_set_degrees,
    device_loop,
};

const SERVO_PERIOD_US: u16 = 20_000; // 20 ms

/// Default minimum pulse width for hobby servos (microseconds).
pub const SERVO_MIN_US_DEFAULT: u16 = 500;

/// Default maximum pulse width for hobby servos (microseconds).
pub const SERVO_MAX_US_DEFAULT: u16 = 2_500;

macro_schema::define! {
    /// Create a servo with keyword arguments and default pulse widths.
    ///
    /// See the [servo module documentation](mod@crate::servo) for details and examples.
    pub servo {
        /// GPIO pin for servo output, for example `PIN_11`; its PWM slice is passed to `new`.
        pin: ident,
        /// Minimum pulse width in microseconds, for 0°.
        #[default_display = "500"]
        min_us: expr = $crate::servo::SERVO_MIN_US_DEFAULT,
        /// Maximum pulse width in microseconds, for `max_degrees`.
        #[default_display = "2500"]
        max_us: expr = $crate::servo::SERVO_MAX_US_DEFAULT,
        /// Maximum servo angle in degrees.
        #[default_display = "180"]
        max_degrees: expr = $crate::servo::ServoRp::DEFAULT_MAX_DEGREES,
        /// Logical angle direction.
        #[default_display = "Direction::Forward"]
        direction: expr = $crate::servo::Direction::Forward,
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            servo: $crate::servo::ServoRp<'static>,
        }

        impl $decl.name {
            /// Creates the servo on the PWM slice that drives its pin.
            ///
            /// `slice` must be the slice for this pin, for example `p.PWM_SLICE5` for `PIN_11`;
            /// see [`ServoPwmSlice`](device_envoy_rp::servo::ServoPwmSlice). See the
            /// [servo module documentation](mod@device_envoy_rp::servo) for usage.
            pub fn new(
                pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pin>>,
                slice: impl $crate::servo::ServoPwmSlice<::embassy_rp::peripherals::$decl.pin>,
            ) -> &'static Self {
                static SERVO_CELL: $crate::__static_cell::StaticCell<$decl.name> =
                    $crate::__static_cell::StaticCell::new();
                SERVO_CELL.init(Self {
                    servo: $crate::servo::servo_from_pin_slice(
                        pin.into(),
                        slice,
                        $decl.min_us,
                        $decl.max_us,
                        $decl.max_degrees,
                        $decl.direction,
                    ),
                })
            }
        }

        impl $crate::servo::Servo for $decl.name {
            const DEFAULT_MAX_DEGREES: u16 = $decl.max_degrees;

            fn set_degrees(&self, degrees: u16) {
                $crate::servo::Servo::set_degrees(&self.servo, degrees);
            }

            fn hold(&self) {
                $crate::servo::Servo::hold(&self.servo);
            }

            fn relax(&self) {
                $crate::servo::Servo::relax(&self.servo);
            }
        }
    }
}

/// The PWM slice for a servo on pin `P`: the bound a servo's `new` puts on its `slice`
/// argument, so passing the wrong slice is a compile error.
///
/// Each slice drives two neighboring pins. GPIO 0–31 use `PWM_SLICE` *n* / 2 mod 8, so
/// `PIN_10` and `PIN_11` use `PWM_SLICE5`, and `PIN_26` uses `PWM_SLICE5` too. On the Pico 2,
/// GPIO 32–47 use `PWM_SLICE8` to `PWM_SLICE11`.
///
/// This trait is sealed: it is implemented for each slice and the two pins it drives, and
/// nothing else.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not the PWM slice for `{P}`",
    label = "expected the PWM slice that drives `{P}`",
    note = "GPIO n uses PWM_SLICE n / 2 mod 8 (GPIO 32-47 on the Pico 2: PWM_SLICE8 to PWM_SLICE11); for example, PIN_10 and PIN_11 use PWM_SLICE5"
)]
pub trait ServoPwmSlice<P: embassy_rp::PeripheralType>: servo_pwm_slice_sealed::Sealed<P> {
    #[doc(hidden)]
    const IS_CHANNEL_A: bool;
    #[doc(hidden)]
    fn new_pwm(self, pin: embassy_rp::Peri<'static, P>) -> Pwm<'static>;
}

// Keeps `ServoPwmSlice` closed to the pin-to-slice map below.
mod servo_pwm_slice_sealed {
    pub trait Sealed<P> {}
}

// Public for macro expansion in downstream crates.
#[doc(hidden)]
pub fn servo_from_pin_slice<P: embassy_rp::PeripheralType, S: ServoPwmSlice<P>>(
    pin: embassy_rp::Peri<'static, P>,
    slice: S,
    min_us: u16,
    max_us: u16,
    max_degrees: u16,
    direction: Direction,
) -> ServoRp<'static> {
    let pwm = slice.new_pwm(pin);
    if S::IS_CHANNEL_A {
        ServoRp::new_output_a(pwm, min_us, max_us, max_degrees, direction)
    } else {
        ServoRp::new_output_b(pwm, min_us, max_us, max_degrees, direction)
    }
}

macro_rules! servo_pin_map {
    ($pin:ident, $slice:ident, $channel_a:literal, $new_output:ident) => {
        impl servo_pwm_slice_sealed::Sealed<embassy_rp::peripherals::$pin>
            for embassy_rp::Peri<'static, embassy_rp::peripherals::$slice>
        {
        }
        impl ServoPwmSlice<embassy_rp::peripherals::$pin>
            for embassy_rp::Peri<'static, embassy_rp::peripherals::$slice>
        {
            const IS_CHANNEL_A: bool = $channel_a;
            fn new_pwm(
                self,
                pin: embassy_rp::Peri<'static, embassy_rp::peripherals::$pin>,
            ) -> Pwm<'static> {
                embassy_rp::pwm::Pwm::$new_output(self, pin, Config::default())
            }
        }
    };
    ($pin:ident, $slice:ident, A) => {
        servo_pin_map!($pin, $slice, true, new_output_a);
    };
    ($pin:ident, $slice:ident, B) => {
        servo_pin_map!($pin, $slice, false, new_output_b);
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
///     servo! {
///         Servo11 { pin: PIN_11 }
///     }
///     let servo = Servo11::new(p.PIN_11, p.PWM_SLICE5);
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
    direction: Direction,
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
    pub(crate) fn new_output_a(
        pwm: Pwm<'d>,
        min_us: u16,
        max_us: u16,
        max_degrees: u16,
        direction: Direction,
    ) -> Self {
        Self::init(pwm, ServoChannel::A, min_us, max_us, max_degrees, direction)
    }

    /// Create a servo on a PWM output B channel.
    ///
    /// See the [servo module documentation](mod@crate::servo) for usage examples.
    pub(crate) fn new_output_b(
        pwm: Pwm<'d>,
        min_us: u16,
        max_us: u16,
        max_degrees: u16,
        direction: Direction,
    ) -> Self {
        Self::init(pwm, ServoChannel::B, min_us, max_us, max_degrees, direction)
    }

    /// Configure PWM and initialize servo. Internal shared logic.
    fn init(
        mut pwm: Pwm<'d>,
        channel: ServoChannel,
        min_us: u16,
        max_us: u16,
        max_degrees: u16,
        direction: Direction,
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
            direction,
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
        let physical_degrees = match self.direction {
            Direction::Forward => degrees,
            Direction::Reverse => self.max_degrees - degrees,
        };
        let us = self.min_us as u32
            + (u32::from(physical_degrees)) * (u32::from(self.max_us) - u32::from(self.min_us))
                / u32::from(self.max_degrees);
        info!(
            "Servo set_degrees(logical={} physical={}) -> {}µs",
            degrees, physical_degrees, us
        );
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
