//! A device abstraction for servo animation control on ESP LEDC PWM.
//!
//! Use [`servo_player!`] for typed servo players.
// TODO0 Document LEDC timer/channel ownership protocol and link-time claim behavior in module docs/README.

use core::borrow::Borrow;

use crate::servo::{Servo, ServoStatic};
use crate::Result;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use heapless::Vec;

const SERVO_PLAYER_MAX_STEPS_CAPACITY: usize = 64;

enum PlayerCommand {
    Set {
        degrees: u16,
    },
    Animate {
        steps: Vec<(u16, Duration), SERVO_PLAYER_MAX_STEPS_CAPACITY>,
        mode: device_envoy_core::servo_player::AtEnd,
    },
    Hold,
    Relax,
}

type PlayerCommandSignal = Signal<CriticalSectionRawMutex, PlayerCommand>;

/// Static resources for one servo player instance.
pub struct ServoPlayerStatic {
    servo_static: ServoStatic,
    command: PlayerCommandSignal,
    max_steps: usize,
}

impl ServoPlayerStatic {
    /// Create static resources for one servo player.
    #[must_use]
    pub const fn new_static(
        timer_number: esp_hal::ledc::timer::Number,
        channel_number: esp_hal::ledc::channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
        max_steps: usize,
    ) -> Self {
        assert!(max_steps <= SERVO_PLAYER_MAX_STEPS_CAPACITY);
        Self {
            servo_static: ServoStatic::new_static(
                timer_number,
                channel_number,
                min_us,
                max_us,
                max_degrees,
            ),
            command: Signal::new(),
            max_steps,
        }
    }
}

/// Servo player handle for background animation commands.
#[derive(Clone, Copy)]
pub struct ServoPlayer {
    command: &'static PlayerCommandSignal,
    max_steps: usize,
}

impl ServoPlayer {
    /// Create static resources.
    #[must_use]
    pub const fn new_static(
        timer_number: esp_hal::ledc::timer::Number,
        channel_number: esp_hal::ledc::channel::Number,
        min_us: u32,
        max_us: u32,
        max_degrees: u16,
        max_steps: usize,
    ) -> ServoPlayerStatic {
        ServoPlayerStatic::new_static(
            timer_number,
            channel_number,
            min_us,
            max_us,
            max_degrees,
            max_steps,
        )
    }

    /// Create a player and create its output device without spawning.
    #[doc(hidden)]
    pub fn new_unspawned(
        servo_player_static: &'static ServoPlayerStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
    ) -> Result<(Self, Servo)> {
        let mut servo = Servo::new(&servo_player_static.servo_static, ledc, pin)?;
        servo.set_degrees(0)?;
        Ok((
            Self {
                command: &servo_player_static.command,
                max_steps: servo_player_static.max_steps,
            },
            servo,
        ))
    }

    /// Create a player and spawn its background task.
    pub fn new(
        servo_player_static: &'static ServoPlayerStatic,
        ledc: &esp_hal::ledc::Ledc<'static>,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        spawner: Spawner,
    ) -> Result<Self> {
        let (servo_player, servo) = Self::new_unspawned(servo_player_static, ledc, pin)?;
        spawner.spawn(servo_player_loop_shared(servo, servo_player_static))?;
        Ok(servo_player)
    }

    /// Set target angle in degrees.
    pub fn set_degrees(&self, degrees: u16) {
        self.command.signal(PlayerCommand::Set { degrees });
    }

    /// Hold current angle.
    pub fn hold(&self) {
        self.command.signal(PlayerCommand::Hold);
    }

    /// Relax (stop drive).
    pub fn relax(&self) {
        self.command.signal(PlayerCommand::Relax);
    }

    /// Animate through step tuples `(degrees, duration)`.
    pub fn animate<I>(&self, steps: I, at_end: device_envoy_core::servo_player::AtEnd)
    where
        I: IntoIterator,
        I::Item: Borrow<(u16, embassy_time::Duration)>,
    {
        assert!(self.max_steps > 0, "animate disabled: max_steps is 0");

        let mut sequence: Vec<(u16, Duration), SERVO_PLAYER_MAX_STEPS_CAPACITY> = Vec::new();
        for step in steps {
            let step = *step.borrow();
            assert!(
                step.1.as_micros() > 0,
                "animation step duration must be positive"
            );
            sequence
                .push(step)
                .expect("animation sequence fits global capacity");
            assert!(
                sequence.len() <= self.max_steps,
                "animation sequence exceeds max_steps"
            );
        }
        assert!(!sequence.is_empty(), "animate requires at least one step");

        self.command.signal(PlayerCommand::Animate {
            steps: sequence,
            mode: at_end,
        });
    }
}

async fn servo_player_loop(mut servo: Servo, command: &'static PlayerCommandSignal) -> ! {
    let mut current_degrees: u16 = 0;
    loop {
        match command.wait().await {
            PlayerCommand::Set { degrees } => {
                servo
                    .set_degrees(degrees)
                    .expect("servo set_degrees failed in servo_player loop");
                current_degrees = degrees;
            }
            PlayerCommand::Hold => {
                servo.hold();
            }
            PlayerCommand::Relax => {
                servo
                    .relax()
                    .expect("servo relax failed in servo_player loop");
            }
            PlayerCommand::Animate { steps, mode } => {
                run_animation(&mut servo, command, &steps, mode, &mut current_degrees).await;
            }
        }
    }
}

#[doc(hidden)]
pub async fn servo_player_task_main(
    servo: Servo,
    servo_player_static: &'static ServoPlayerStatic,
) -> ! {
    servo_player_loop(servo, &servo_player_static.command).await
}

#[embassy_executor::task(pool_size = 8)]
async fn servo_player_loop_shared(
    servo: Servo,
    servo_player_static: &'static ServoPlayerStatic,
) -> ! {
    servo_player_task_main(servo, servo_player_static).await
}

async fn run_animation(
    servo: &mut Servo,
    command: &'static PlayerCommandSignal,
    steps: &[(u16, Duration)],
    mode: device_envoy_core::servo_player::AtEnd,
    current_degrees: &mut u16,
) {
    loop {
        for step in steps {
            if *current_degrees != step.0 {
                servo
                    .set_degrees(step.0)
                    .expect("servo set_degrees failed in servo animation");
                *current_degrees = step.0;
            }

            match select(Timer::after(step.1), command.wait()).await {
                Either::First(_) => {}
                Either::Second(new_command) => {
                    command.signal(new_command);
                    return;
                }
            }
        }

        match mode {
            device_envoy_core::servo_player::AtEnd::Loop => {}
            device_envoy_core::servo_player::AtEnd::Hold => return,
            device_envoy_core::servo_player::AtEnd::Relax => {
                servo
                    .relax()
                    .expect("servo relax failed at end of animation");
                return;
            }
        }
    }
}

#[doc(hidden)]
pub use paste;

/// Create a typed servo player with keyword configuration.
#[macro_export]
#[doc(hidden)]
macro_rules! servo_player {
    ($($tt:tt)*) => { $crate::__servo_player_impl! { $($tt)* } };
}
#[doc(inline)]
pub use servo_player;

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __servo_player_impl {
    (
        $name:ident {
            timer: $timer:ident,
            channel: $channel:ident,
            $(min_us: $min_us:expr,)?
            $(max_us: $max_us:expr,)?
            $(max_degrees: $max_degrees:expr,)?
            $(max_steps: $max_steps:expr $(,)?)?
        }
    ) => {
        $crate::servo_player::paste::paste! {
            pub struct $name;

            // Link-time ownership claims: duplicate timer or channel selection across the
            // final binary should fail the link with duplicate symbol errors.
            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_timer_claim_ $timer:lower>]: u8 = 0;

            #[used]
            #[unsafe(no_mangle)]
            static [<__device_envoy_esp_ledc_channel_claim_ $channel:lower>]: u8 = 0;

            static [<$name:upper _SERVO_PLAYER_STATIC>]: [<$name Static>] = $name::new_static();

            pub struct [<$name Static>] {
                servo_player_static: $crate::servo_player::ServoPlayerStatic,
            }

            impl $name {
                pub const MAX_STEPS: usize = $crate::__servo_player_impl!(@max_steps $($max_steps)?);

                #[must_use]
                pub const fn new_static() -> [<$name Static>] {
                    [<$name Static>] {
                        servo_player_static: $crate::servo_player::ServoPlayer::new_static(
                            ::esp_hal::ledc::timer::Number::$timer,
                            ::esp_hal::ledc::channel::Number::$channel,
                            $crate::__servo_player_impl!(@min_us $($min_us)?),
                            $crate::__servo_player_impl!(@max_us $($max_us)?),
                            $crate::__servo_player_impl!(@max_degrees $($max_degrees)?),
                            Self::MAX_STEPS,
                        ),
                    }
                }

                pub fn new_with_static(
                    servo_player_static: &'static [<$name Static>],
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: impl ::esp_hal::gpio::interconnect::PeripheralOutput<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<$crate::servo_player::ServoPlayer> {
                    let (servo_player, servo) = $crate::servo_player::ServoPlayer::new_unspawned(
                        &servo_player_static.servo_player_static,
                        ledc,
                        pin,
                    )?;
                    spawner.spawn([<__ $name:snake _servo_player_task>](
                        servo,
                        &servo_player_static.servo_player_static,
                    ))?;
                    Ok(servo_player)
                }

                pub fn new(
                    ledc: &::esp_hal::ledc::Ledc<'static>,
                    pin: impl ::esp_hal::gpio::interconnect::PeripheralOutput<'static>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<$crate::servo_player::ServoPlayer> {
                    Self::new_with_static(&[<$name:upper _SERVO_PLAYER_STATIC>], ledc, pin, spawner)
                }
            }

            #[::embassy_executor::task]
            async fn [<__ $name:snake _servo_player_task>](
                servo: $crate::servo::Servo,
                servo_player_static: &'static $crate::servo_player::ServoPlayerStatic,
            ) -> ! {
                $crate::servo_player::servo_player_task_main(servo, servo_player_static).await
            }
        }
    };

    (@min_us $min_us:expr) => { $min_us };
    (@min_us) => { $crate::servo::SERVO_MIN_US_DEFAULT };
    (@max_us $max_us:expr) => { $max_us };
    (@max_us) => { $crate::servo::SERVO_MAX_US_DEFAULT };
    (@max_degrees $max_degrees:expr) => { $max_degrees };
    (@max_degrees) => { $crate::servo::Servo::DEFAULT_MAX_DEGREES };
    (@max_steps $max_steps:expr) => { $max_steps };
    (@max_steps) => { 16 };
}

#[doc(inline)]
pub use device_envoy_core::servo_player::{combine, linear, AtEnd};
