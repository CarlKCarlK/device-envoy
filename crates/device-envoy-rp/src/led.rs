//! A device abstraction for a single digital LED with animation support.
//!
//! Use the [`led!`](macro@crate::led) macro to generate one or more concrete LED
//! device types.
//!
//! See [`LedGenerated`](led_generated::LedGenerated) for a sample generated type.
//!
//! # Example
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::future::pending;
//! use device_envoy_rp::{
//!     Result,
//!     led,
//!     led::{Led as _, LedLevel, OnLevel},
//! };
//! use embassy_time::Duration;
//! # #[panic_handler]
//! # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
//!
//! led!(pub LedOne { pin: PIN_1 });
//! led!(pub LedTwo {
//!     pin: PIN_2,
//!     max_steps: 2,
//! });
//!
//! async fn example(p: embassy_rp::Peripherals, spawner: embassy_executor::Spawner) -> Result<()> {
//!     let led_one = LedOne::new(p.PIN_1, OnLevel::High, spawner)?;
//!     let led_two = LedTwo::new(p.PIN_2, OnLevel::High, spawner)?;
//!
//!     led_one.set_level(LedLevel::On);
//!     led_two.set_level(LedLevel::Off);
//!     embassy_time::Timer::after(Duration::from_millis(250)).await;
//!
//!     led_one.animate([
//!         (LedLevel::On, Duration::from_millis(200)),
//!         (LedLevel::Off, Duration::from_millis(200)),
//!     ]);
//!     led_two.animate([
//!         (LedLevel::Off, Duration::from_millis(150)),
//!         (LedLevel::On, Duration::from_millis(150)),
//!     ]);
//!
//!     pending().await
//! }
//! ```

use embassy_rp::gpio::{Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use heapless::Vec;

pub use device_envoy_core::led::{Led, LedLevel, OnLevel};
pub mod led_generated;
#[cfg(not(feature = "host"))]
#[doc(hidden)]
pub use paste;

/// Maximum number of animation frames allowed.
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub const DEFAULT_MAX_STEPS: usize = 32;

#[derive(Clone)]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub enum LedCommand<const MAX_STEPS: usize> {
    /// Set LED level immediately.
    Set(LedLevel),
    /// Play an animation sequence (looping).
    Animate(Vec<(LedLevel, Duration), MAX_STEPS>),
}

/// Signal for sending LED commands to macro-generated LED devices.
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub type LedOuterStatic<const MAX_STEPS: usize> =
    Signal<CriticalSectionRawMutex, LedCommand<MAX_STEPS>>;

/// Static resources for a macro-generated LED device.
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub struct LedStatic<const MAX_STEPS: usize> {
    outer: LedOuterStatic<MAX_STEPS>,
}

impl<const MAX_STEPS: usize> LedStatic<MAX_STEPS> {
    /// Creates static resources for a single LED device.
    #[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
    pub const fn new() -> Self {
        Self {
            outer: Signal::new(),
        }
    }

    #[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
    pub fn outer(&self) -> &LedOuterStatic<MAX_STEPS> {
        &self.outer
    }
}

/// Set the physical pin state based on desired LED level and on_level.
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub fn set_pin_for_led_level(led_level: LedLevel, pin: &mut Output<'_>, on_level: OnLevel) {
    let pin_level = match (led_level, on_level) {
        (LedLevel::On, OnLevel::High) | (LedLevel::Off, OnLevel::Low) => Level::High,
        (LedLevel::Off, OnLevel::High) | (LedLevel::On, OnLevel::Low) => Level::Low,
    };
    pin.set_level(pin_level);
}

#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub async fn run_set_level_loop<const MAX_STEPS: usize>(
    led_level: LedLevel,
    outer_static: &'static LedOuterStatic<MAX_STEPS>,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand<MAX_STEPS> {
    set_pin_for_led_level(led_level, pin, on_level);

    loop {
        match outer_static.wait().await {
            LedCommand::Set(new_led_level) => {
                if new_led_level == led_level {
                    continue;
                }
                return LedCommand::Set(new_led_level);
            }
            other => return other,
        }
    }
}

#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub async fn run_animation_loop<const MAX_STEPS: usize>(
    animation: Vec<(LedLevel, Duration), MAX_STEPS>,
    outer_static: &'static LedOuterStatic<MAX_STEPS>,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand<MAX_STEPS> {
    if animation.is_empty() {
        return LedCommand::Animate(animation);
    }

    let mut frame_index = 0;

    loop {
        let (led_level, duration) = animation[frame_index];

        set_pin_for_led_level(led_level, pin, on_level);

        frame_index = (frame_index + 1) % animation.len();

        match embassy_futures::select::select(Timer::after(duration), outer_static.wait()).await {
            embassy_futures::select::Either::First(_) => {}
            embassy_futures::select::Either::Second(command) => {
                return command;
            }
        }
    }
}

/// Macro to generate a single LED struct type (includes syntax details).
///
/// **See the [led module documentation](mod@crate::led) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// led! {
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///         max_steps: <usize_expr>, // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO pin resource type for this generated LED.
///
/// **Optional fields:**
///
/// - `max_steps` — Maximum number of animation frames (default: 32).
///
/// `max_steps = 0` disables animation storage; `set_level()` is still supported.
#[doc(hidden)]
#[macro_export]
macro_rules! led {
    // TODO_NIGHTLY When nightly feature `decl_macro` becomes stable, change this
    // code by replacing `#[macro_export] macro_rules!` with module-scoped `pub macro`
    // so macro visibility and helper exposure can be controlled more precisely.
    ($($tt:tt)*) => { $crate::__led_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led!`] instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_impl {
    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led_impl! {
            @__parse
            vis: $vis,
            name: $name,
            pin: [],
            max_steps: [],
            fields: [ $($fields)* ]
        }
    };

    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [],
        max_steps: [$($max_steps:expr)?],
        fields: [ pin: $pin:ident $(, $($rest:tt)*)? ]
    ) => {
        $crate::__led_impl! {
            @__parse
            vis: $vis,
            name: $name,
            pin: [$pin],
            max_steps: [$($max_steps)?],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [$_pin_seen:ident],
        max_steps: [$($max_steps:expr)?],
        fields: [ pin: $pin:ident $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("led! duplicate `pin` field");
    };

    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [$($pin:ident)?],
        max_steps: [],
        fields: [ max_steps: $max_steps:expr $(, $($rest:tt)*)? ]
    ) => {
        $crate::__led_impl! {
            @__parse
            vis: $vis,
            name: $name,
            pin: [$($pin)?],
            max_steps: [$max_steps],
            fields: [ $($($rest)*)? ]
        }
    };
    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [$($pin:ident)?],
        max_steps: [$_max_steps_seen:expr],
        fields: [ max_steps: $max_steps:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("led! duplicate `max_steps` field");
    };

    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [$($pin:ident)?],
        max_steps: [$($max_steps:expr)?],
        fields: [ ]
    ) => {
        $crate::__led_impl! {
            @__finish
            vis: $vis,
            name: $name,
            pin: [$($pin)?],
            max_steps: [$($max_steps)?]
        }
    };

    (@__parse
        vis: $vis:vis,
        name: $name:ident,
        pin: [$($pin:ident)?],
        max_steps: [$($max_steps:expr)?],
        fields: [ $field:ident : $value:expr $(, $($rest:tt)*)? ]
    ) => {
        compile_error!("led! unknown field; expected `pin` or `max_steps`");
    };

    (@__finish
        vis: $vis:vis,
        name: $name:ident,
        pin: [],
        max_steps: [$($max_steps:expr)?]
    ) => {
        compile_error!("led! missing required `pin` field");
    };

    (@__finish
        vis: $vis:vis,
        name: $name:ident,
        pin: [$pin:ident],
        max_steps: []
    ) => {
        $crate::__led_impl!(@__emit vis: $vis, name: $name, pin: $pin, max_steps: $crate::led::DEFAULT_MAX_STEPS);
    };

    (@__finish
        vis: $vis:vis,
        name: $name:ident,
        pin: [$pin:ident],
        max_steps: [$max_steps:expr]
    ) => {
        $crate::__led_impl!(@__emit vis: $vis, name: $name, pin: $pin, max_steps: $max_steps);
    };

    (
        @__emit
        vis: $vis:vis,
        name: $name:ident,
        pin: $pin:ident,
        max_steps: $max_steps:expr
    ) => {
        $crate::led::paste::paste! {
            const [<$name:upper _MAX_STEPS>]: usize = $max_steps;

            #[allow(non_upper_case_globals)]
            static [<$name:upper _STATIC>]: $crate::led::LedStatic<{ [<$name:upper _MAX_STEPS>] }> =
                $crate::led::LedStatic::new();

            #[allow(non_camel_case_types)]
            $vis struct $name(&'static $crate::led::LedOuterStatic<{ [<$name:upper _MAX_STEPS>] }>);

            impl $name {
                $vis const MAX_STEPS: usize = [<$name:upper _MAX_STEPS>];

                $vis fn new(
                    pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                    on_level: $crate::led::OnLevel,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<Self> {
                    let pin_output = ::embassy_rp::gpio::Output::new(pin, ::embassy_rp::gpio::Level::Low);
                    let token = [<__led_task_ $name:snake>](
                        [<$name:upper _STATIC>].outer(),
                        pin_output,
                        on_level,
                    );
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                    Ok(Self([<$name:upper _STATIC>].outer()))
                }
            }

            impl $crate::led::Led for $name {
                fn set_level(&self, led_level: $crate::led::LedLevel) {
                    self.0.signal($crate::led::LedCommand::Set(led_level));
                }

                fn animate<I>(&self, frames: I)
                where
                    I: IntoIterator,
                    I::Item: ::core::borrow::Borrow<(
                        $crate::led::LedLevel,
                        ::embassy_time::Duration,
                    )>,
                {
                    let mut animation: ::heapless::Vec<
                        ($crate::led::LedLevel, ::embassy_time::Duration),
                        { [<$name:upper _MAX_STEPS>] },
                    > = ::heapless::Vec::new();
                    for frame in frames {
                        let frame = *::core::borrow::Borrow::borrow(&frame);
                        animation
                            .push(frame)
                            .expect("LED animation fits within MAX_STEPS");
                    }
                    self.0.signal($crate::led::LedCommand::Animate(animation));
                }
            }

            #[::embassy_executor::task]
            async fn [<__led_task_ $name:snake>](
                outer_static: &'static $crate::led::LedOuterStatic<{ [<$name:upper _MAX_STEPS>] }>,
                mut pin: ::embassy_rp::gpio::Output<'static>,
                on_level: $crate::led::OnLevel,
            ) -> ! {
                let mut command = $crate::led::LedCommand::Set($crate::led::LedLevel::Off);
                $crate::led::set_pin_for_led_level($crate::led::LedLevel::Off, &mut pin, on_level);

                loop {
                    command = match command {
                        $crate::led::LedCommand::Set(led_level) => {
                            $crate::led::run_set_level_loop(led_level, outer_static, &mut pin, on_level).await
                        }
                        $crate::led::LedCommand::Animate(animation) => {
                            $crate::led::run_animation_loop(animation, outer_static, &mut pin, on_level).await
                        }
                    };
                }
            }
        }
    };
}

#[doc(inline)]
pub use led;
