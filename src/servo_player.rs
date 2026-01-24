//! A device abstraction for playing servo motion sequences.
//!
//! See [`ServoPlayer`] for usage and examples.

use core::array;

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;

use crate::servo::Servo;

pub use crate::servo::servo;
#[doc(hidden)]
pub use paste;

/// Commands sent to the servo player device.
enum PlayerCommand<const MAX_STEPS: usize> {
    Set {
        degrees: u16,
    },
    Animate {
        steps: Vec<(u16, Duration), MAX_STEPS>,
        mode: AnimateMode,
    },
}

/// Animation end behavior.
#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum AnimateMode {
    /// Repeat the animation sequence indefinitely.
    Loop,
    /// Hold the final position when animation completes.
    Stay,
    /// Disable PWM after animation completes (servo relaxes).
    Relax,
}

/// Build a linear sequence of animation steps from `start_degrees` to `end_degrees` over
/// `total_duration` split into `N` steps (inclusive of endpoints).
///
/// Each step is a tuple `(degrees, duration)`.
///
/// See the [struct-level example](ServoPlayer) for usage.
#[must_use]
pub fn linear<const N: usize>(
    start_degrees: u16,
    end_degrees: u16,
    total_duration: Duration,
) -> [(u16, Duration); N] {
    assert!(N > 0, "at least one step required");
    assert!(
        total_duration.as_micros() > 0,
        "total duration must be positive"
    );
    let step_duration = total_duration / (N as u32);
    let delta = i32::from(end_degrees) - i32::from(start_degrees);
    let denom = i32::try_from(((N - 1) as i32).max(1)).expect("denom fits in i32");
    array::from_fn(|step_index| {
        let degrees = if N == 1 {
            start_degrees
        } else {
            let step_delta = delta * i32::try_from(step_index).expect("index fits") / denom;
            u16::try_from(i32::from(start_degrees) + step_delta).expect("angle fits")
        };
        (degrees, step_duration)
    })
}

/// Concatenate arrays of animation steps into a single sequence.
///
/// Each step is a tuple `(degrees, duration)`.
///
/// See the [struct-level example](ServoPlayer) for usage.
#[must_use]
pub fn concat_steps<const CAP: usize>(
    sequences: &[&[(u16, Duration)]],
) -> Vec<(u16, Duration), CAP> {
    let mut out: Vec<(u16, Duration), CAP> = Vec::new();
    for sequence in sequences {
        for step in *sequence {
            out.push(*step).expect("sequence fits");
        }
    }
    out
}

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

/// A device abstraction that drives a single servo with scripted animation sequences.
///
/// See [`Servo`] for servo setup guidance.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_kit::servo_player::{AnimateMode, concat_steps, linear, servo_player};
/// use embassy_time::Duration;
/// # use core::panic::PanicInfo;
/// # #[panic_handler]
/// # fn panic(_info: &PanicInfo) -> ! { loop {} }
///
/// servo_player! {
///     ServoSweep {
///         pin: PIN_11,
///     }
/// }
///
/// async fn demo(p: embassy_rp::Peripherals, spawner: embassy_executor::Spawner) {
///     let servo_sweep = ServoSweep::new(p.PIN_11, p.PWM_SLICE5, spawner).unwrap();
///
///     const SWEEP_SECONDS: Duration = Duration::from_secs(2);
///     let sweep = linear::<11>(0, 180, SWEEP_SECONDS);
///     let sequence = concat_steps::<16>(&[&sweep]);
///     servo_sweep.animate(sequence, AnimateMode::Loop);
/// }
/// ```
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
    /// See the [struct-level example](Self) for usage.
    #[must_use]
    pub const fn new(servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>) -> Self {
        Self {
            servo_player_static,
        }
    }

    /// Set the target angle. The most recent command always wins.
    ///
    /// See the [struct-level example](Self) for usage.
    pub fn set(&self, degrees: u16) {
        self.servo_player_static
            .signal(PlayerCommand::Set { degrees });
    }

    /// Animate the servo through a sequence of angles with per-step hold durations.
    ///
    /// Each step is a tuple `(degrees, duration)`.
    ///
    /// See the [struct-level example](Self) for usage.
    pub fn animate(&self, steps: impl IntoIterator<Item = (u16, Duration)>, mode: AnimateMode) {
        assert!(MAX_STEPS > 0, "animate disabled: max_steps is 0");
        let mut sequence: Vec<(u16, Duration), MAX_STEPS> = Vec::new();
        for step in steps {
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
            mode,
        });
    }
}

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
    mode: AnimateMode,
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
            AnimateMode::Loop => {
                // Continue looping
            }
            AnimateMode::Stay => {
                // Hold final position and wait for next command
                return servo_player_static.wait().await;
            }
            AnimateMode::Relax => {
                // Disable PWM (servo relaxes) and wait for next command
                servo.disable();
                return servo_player_static.wait().await;
            }
        }
    }
}
