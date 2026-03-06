//! A device abstraction for servo animation control primitives shared across platforms.
//!
//! This module provides the platform-independent command engine used by
//! platform crates to build servo-player APIs.

#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::borrow::Borrow;

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;

use crate::servo::Servo as ServoTrait;

/// Commands sent to the servo player device loop.
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AtEnd {
    /// Repeat the animation sequence indefinitely.
    Loop,
    /// Hold the final position when animation completes.
    Hold,
    /// Stop holding position after animation completes (servo relaxes).
    Relax,
}

/// Build a const linear sequence of animation steps as an array.
///
/// This uses [`embassy_time::Duration`] for step timing.
#[must_use]
pub const fn linear<const N: usize>(
    start_degrees: u16,
    end_degrees: u16,
    total_duration: embassy_time::Duration,
) -> [(u16, embassy_time::Duration); N] {
    assert!(N > 0, "at least one step required");
    let step_duration = Duration::from_micros(total_duration.as_micros() / (N as u64));
    let delta = end_degrees as i32 - start_degrees as i32;
    let denom = if N == 1 { 1 } else { (N - 1) as i32 };

    let mut result = [(0u16, Duration::from_micros(0)); N];
    let mut step_index = 0;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
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
/// This uses [`embassy_time::Duration`] for step timing.
#[must_use]
pub const fn combine<const N1: usize, const N2: usize, const OUT_N: usize>(
    first: [(u16, embassy_time::Duration); N1],
    second: [(u16, embassy_time::Duration); N2],
) -> [(u16, embassy_time::Duration); OUT_N] {
    assert!(OUT_N == N1 + N2, "OUT_N must equal N1 + N2");

    let mut result = [(0u16, Duration::from_micros(0)); OUT_N];
    let mut first_index = 0;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace these while loops with for loops.
    while first_index < N1 {
        result[first_index] = first[first_index];
        first_index += 1;
    }
    let mut second_index = 0;
    while second_index < N2 {
        result[N1 + second_index] = second[second_index];
        second_index += 1;
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

/// Internal servo-player command handle used by platform macro-generated types.
///
/// This must remain `pub` because `servo_player!` macro expansions in platform crates
/// construct this handle type.
#[doc(hidden)]
pub struct ServoPlayerHandle<const MAX_STEPS: usize> {
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
}

impl<const MAX_STEPS: usize> ServoPlayerHandle<MAX_STEPS> {
    /// Create static resources for a servo player.
    #[must_use]
    pub const fn new_static() -> ServoPlayerStatic<MAX_STEPS> {
        ServoPlayerStatic::new_static()
    }

    /// Create a servo player handle. The device loop must already be running.
    #[must_use]
    pub const fn new(servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>) -> Self {
        Self {
            servo_player_static,
        }
    }
}

/// Platform-agnostic servo-player device contract.
///
/// Platform crates implement this trait for generated servo player types so servo
/// operations resolve through trait methods instead of inherent methods.
pub trait ServoPlayer<const MAX_STEPS: usize> {
    /// Maximum number of animation steps accepted by [`ServoPlayer::animate`].
    const MAX_STEPS: usize;

    /// Set the target angle. The most recent command always wins.
    fn set_degrees(&self, degrees: u16);

    /// Hold the servo at its current position.
    fn hold(&self);

    /// Relax the servo (stops holding position, servo can move freely).
    fn relax(&self);

    /// Animate through a sequence of angles with per-step hold durations.
    ///
    /// This uses [`embassy_time::Duration`] for step timing.
    fn animate<I>(&self, steps: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<(u16, embassy_time::Duration)>;
}

// Must remain `pub` because platform-crate macro expansions call this helper.
#[doc(hidden)]
pub fn __servo_player_set_degrees<const MAX_STEPS: usize>(
    servo_player_handle: &ServoPlayerHandle<MAX_STEPS>,
    degrees: u16,
) {
    servo_player_handle
        .servo_player_static
        .signal(PlayerCommand::Set { degrees });
}

// Must remain `pub` because platform-crate macro expansions call this helper.
#[doc(hidden)]
pub fn __servo_player_hold<const MAX_STEPS: usize>(
    servo_player_handle: &ServoPlayerHandle<MAX_STEPS>,
) {
    servo_player_handle
        .servo_player_static
        .signal(PlayerCommand::Hold);
}

// Must remain `pub` because platform-crate macro expansions call this helper.
#[doc(hidden)]
pub fn __servo_player_relax<const MAX_STEPS: usize>(
    servo_player_handle: &ServoPlayerHandle<MAX_STEPS>,
) {
    servo_player_handle
        .servo_player_static
        .signal(PlayerCommand::Relax);
}

// Must remain `pub` because platform-crate macro expansions call this helper.
#[doc(hidden)]
pub fn __servo_player_animate<I, const MAX_STEPS: usize>(
    servo_player_handle: &ServoPlayerHandle<MAX_STEPS>,
    steps: I,
    at_end: AtEnd,
) where
    I: IntoIterator,
    I::Item: Borrow<(u16, embassy_time::Duration)>,
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

    servo_player_handle
        .servo_player_static
        .signal(PlayerCommand::Animate {
            steps: sequence,
            mode: at_end,
        });
}

impl<const MAX_STEPS: usize> ServoPlayer<MAX_STEPS> for ServoPlayerHandle<MAX_STEPS> {
    const MAX_STEPS: usize = MAX_STEPS;

    fn set_degrees(&self, degrees: u16) {
        __servo_player_set_degrees(self, degrees);
    }

    fn hold(&self) {
        __servo_player_hold(self);
    }

    fn relax(&self) {
        __servo_player_relax(self);
    }

    fn animate<I>(&self, steps: I, at_end: AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<(u16, embassy_time::Duration)>,
    {
        __servo_player_animate(self, steps, at_end);
    }
}

/// Shared command loop for servo-player devices.
pub async fn device_loop<const MAX_STEPS: usize, O>(
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
    mut servo_player_output: O,
) -> !
where
    O: ServoTrait,
{
    let mut current_degrees: u16 = 0;
    servo_player_output.set_degrees(current_degrees);

    let mut command = servo_player_static.wait().await;
    loop {
        match command {
            PlayerCommand::Set { degrees } => {
                current_degrees = degrees;
                servo_player_output.set_degrees(current_degrees);
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Hold => {
                servo_player_output.hold();
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Relax => {
                servo_player_output.relax();
                command = servo_player_static.wait().await;
            }
            PlayerCommand::Animate { steps, mode } => {
                command = run_animation(
                    &steps,
                    mode,
                    &mut servo_player_output,
                    servo_player_static,
                    &mut current_degrees,
                )
                .await;
            }
        }
    }
}

async fn run_animation<const MAX_STEPS: usize, O>(
    steps: &[(u16, Duration)],
    mode: AtEnd,
    servo_player_output: &mut O,
    servo_player_static: &'static ServoPlayerStatic<MAX_STEPS>,
    current_degrees: &mut u16,
) -> PlayerCommand<MAX_STEPS>
where
    O: ServoTrait,
{
    loop {
        for step in steps {
            if *current_degrees != step.0 {
                servo_player_output.set_degrees(step.0);
                *current_degrees = step.0;
            }
            match select(Timer::after(step.1), servo_player_static.wait()).await {
                Either::First(_) => {}
                Either::Second(command) => return command,
            }
        }

        match mode {
            AtEnd::Loop => {}
            AtEnd::Hold => return servo_player_static.wait().await,
            AtEnd::Relax => {
                servo_player_output.relax();
                return servo_player_static.wait().await;
            }
        }
    }
}
