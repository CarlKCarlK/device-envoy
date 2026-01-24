//! A device abstraction for playing servo motion sequences.
//!
//! See [`ServoPlayer`] for usage and examples.

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;
// cmk00 should led*.animate use borrow?

use core::borrow::Borrow;

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
        mode: AtEnd,
    },
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

/// Iterator that generates a linear sequence of animation steps from `start_degrees` to `end_degrees`.
///
/// Create with [`linear()`].
#[derive(Clone, Debug)]
pub struct Linear {
    current_step: usize,
    total_steps: usize,
    start_degrees: u16,
    end_degrees: u16,
    step_duration: Duration,
}

impl Iterator for Linear {
    type Item = (u16, Duration);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_step >= self.total_steps {
            return None;
        }

        let degrees = if self.total_steps == 1 {
            self.start_degrees
        } else {
            let delta = i32::from(self.end_degrees) - i32::from(self.start_degrees);
            let denom = (self.total_steps - 1) as i32;
            let step_delta = delta * (self.current_step as i32) / denom;
            u16::try_from(i32::from(self.start_degrees) + step_delta).expect("angle fits")
        };

        self.current_step += 1;
        Some((degrees, self.step_duration))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_steps.saturating_sub(self.current_step);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Linear {}

/// Build a linear sequence of animation steps from `start_degrees` to `end_degrees`
/// split into `steps` (inclusive of endpoints) over `total_duration`.
///
/// Returns a [`Linear`] iterator that yields `(degrees, duration)` tuples.
///
/// See the [struct-level example](ServoPlayer) for usage.
#[must_use]
pub fn linear(
    start_degrees: u16,
    end_degrees: u16,
    steps: usize,
    total_duration: Duration,
) -> Linear {
    assert!(steps > 0, "at least one step required");
    assert!(
        total_duration.as_micros() > 0,
        "total duration must be positive"
    );
    let step_duration = total_duration / (steps as u32);
    Linear {
        current_step: 0,
        total_steps: steps,
        start_degrees,
        end_degrees,
        step_duration,
    }
}

// cmk000 Kill?
/// Build a const linear sequence of animation steps as an array.
///
/// Unlike [`linear()`], this returns a fixed-size array and can be used in const contexts.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::Duration;
/// # use device_kit::servo_player::linear_array;
/// # use panic_probe as _;
/// const SWEEP: [(u16, Duration); 11] = linear_array(0, 180, Duration::from_secs(2));
/// ```
#[must_use]
pub const fn linear_array<const N: usize>(
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

/// Concatenate two animation step arrays into one larger array.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use embassy_time::Duration;
/// # use device_kit::servo_player::{concat_arrays, linear_array};
/// # use panic_probe as _;
/// const SWEEP_UP: [(u16, Duration); 19] = linear_array(0, 180, Duration::from_secs(2));
/// const HOLD: [(u16, Duration); 1] = [(180, Duration::from_millis(400))];
/// const COMBINED: [(u16, Duration); 20] = concat_arrays(SWEEP_UP, HOLD);
/// ```
#[must_use]
pub const fn concat_arrays<const N1: usize, const N2: usize, const OUT_N: usize>(
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
/// use device_kit::servo_player::{AtEnd, linear, servo_player};
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
///     servo_sweep.animate(linear(0, 180, 11, SWEEP_SECONDS), AtEnd::Loop);
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

    // cmk0000 must be consistent with servo
    /// Set the target angle. The most recent command always wins.
    ///
    /// See the [struct-level example](Self) for usage.
    pub fn set_degrees(&self, degrees: u16) {
        self.servo_player_static
            .signal(PlayerCommand::Set { degrees });
    }

    /// Animate the servo through a sequence of angles with per-step hold durations.
    ///
    /// Each step is a tuple `(degrees, duration)`. Accepts both owned iterators and
    /// references to collections.
    ///
    /// See the [struct-level example](Self) for usage.
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
                servo.disable();
                return servo_player_static.wait().await;
            }
        }
    }
}
