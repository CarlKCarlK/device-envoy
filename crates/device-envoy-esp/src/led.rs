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
//! use device_envoy_esp::{
//!     Result,
//!     init_and_start,
//!     led,
//!     led::{Led as _, LedLevel, OnLevel},
//! };
//! use embassy_time::Duration;
//! # #[panic_handler]
//! # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
//!
//! led! {
//!     pub LedOne {
//!         pin: GPIO2
//!     }
//! }
//! led! {
//!     pub LedTwo {
//!         pin: GPIO3,
//!         max_steps: 2
//!     }
//! }
//!
//! async fn example(spawner: embassy_executor::Spawner) -> Result<()> {
//!     init_and_start!(p);
//!     let led_one = LedOne::new(p.GPIO2, OnLevel::High, spawner)?;
//!     let led_two = LedTwo::new(p.GPIO3, OnLevel::High, spawner)?;
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
//!     core::future::pending().await
//! }
//! ```

pub use device_envoy_core::led::{Led, LedLevel, OnLevel};
pub mod led_generated;

#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};
#[cfg(target_os = "none")]
use esp_hal::gpio::{Level, Output};
#[cfg(target_os = "none")]
use heapless::Vec;

#[cfg(target_os = "none")]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub const DEFAULT_MAX_STEPS: usize = 32;

#[cfg(target_os = "none")]
#[derive(Clone)]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub enum LedCommand<const MAX_STEPS: usize> {
    Set(LedLevel),
    Animate(Vec<(LedLevel, Duration), MAX_STEPS>),
}

#[cfg(target_os = "none")]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub type LedOuterStatic<const MAX_STEPS: usize> =
    Signal<CriticalSectionRawMutex, LedCommand<MAX_STEPS>>;

#[cfg(target_os = "none")]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub struct LedStatic<const MAX_STEPS: usize> {
    outer: LedOuterStatic<MAX_STEPS>,
}

#[cfg(target_os = "none")]
impl<const MAX_STEPS: usize> LedStatic<MAX_STEPS> {
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

#[cfg(target_os = "none")]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub fn set_pin_for_led_level(led_level: LedLevel, pin: &mut Output<'_>, on_level: OnLevel) {
    let pin_level = match (led_level, on_level) {
        (LedLevel::On, OnLevel::High) | (LedLevel::Off, OnLevel::Low) => Level::High,
        (LedLevel::Off, OnLevel::High) | (LedLevel::On, OnLevel::Low) => Level::Low,
    };
    pin.set_level(pin_level);
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
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
            embassy_futures::select::Either::Second(command) => return command,
        }
    }
}

#[cfg(target_os = "none")]
const_structures::define! {
    /// Macro to generate a single LED struct type.
    ///
    /// **See the [led module documentation](mod@crate::led) for usage examples.**
    ///
    /// `max_steps = 0` disables animation storage; `set_level()` is still supported.
    pub led {
        /// GPIO pin for the LED.
        pin: ident,
        /// Maximum number of animation steps; `0` disables animation storage.
        #[default_display = "32"]
        max_steps: expr = $crate::led::DEFAULT_MAX_STEPS,
    }

    generate {
        #[cfg(target_os = "none")]
        const $upper($decl.name, _MAX_STEPS): usize = $decl.max_steps;

        #[cfg(target_os = "none")]
        #[allow(non_upper_case_globals)]
        static $upper($decl.name, _STATIC): $crate::led::LedStatic<{ $upper($decl.name, _MAX_STEPS) }> =
            $crate::led::LedStatic::new();

        #[cfg(target_os = "none")]
        #[allow(non_camel_case_types)]
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name(&'static $crate::led::LedOuterStatic<{ $upper($decl.name, _MAX_STEPS) }>);

        #[cfg(target_os = "none")]
        impl $decl.name {
            $decl.vis const MAX_STEPS: usize = $upper($decl.name, _MAX_STEPS);

            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                on_level: $crate::led::OnLevel,
                spawner: embassy_executor::Spawner,
            ) -> $crate::Result<Self> {
                let pin_output = $crate::esp_hal::gpio::Output::new(
                    pin,
                    $crate::esp_hal::gpio::Level::Low,
                    $crate::esp_hal::gpio::OutputConfig::default(),
                );
                let token = $snake(__led_task_, $decl.name)(
                    $upper($decl.name, _STATIC).outer(),
                    pin_output,
                    on_level,
                );
                spawner.spawn(token.map_err($crate::Error::TaskSpawn)?);
                Ok(Self($upper($decl.name, _STATIC).outer()))
            }
        }

        #[cfg(target_os = "none")]
        impl $crate::led::Led for $decl.name {
            fn set_level(&self, led_level: $crate::led::LedLevel) {
                self.0.signal($crate::led::LedCommand::Set(led_level));
            }

            fn animate<I>(&self, frames: I)
            where
                I: IntoIterator,
                I::Item: ::core::borrow::Borrow<(
                    $crate::led::LedLevel,
                    embassy_time::Duration,
                )>,
            {
                let mut animation: heapless::Vec<
                    ($crate::led::LedLevel, embassy_time::Duration),
                    { $upper($decl.name, _MAX_STEPS) },
                > = heapless::Vec::new();
                for frame in frames {
                    let frame = *::core::borrow::Borrow::borrow(&frame);
                    animation
                        .push(frame)
                        .expect("LED animation fits within MAX_STEPS");
                }
                self.0.signal($crate::led::LedCommand::Animate(animation));
            }
        }

        #[cfg(target_os = "none")]
        #[embassy_executor::task]
        async fn $snake(__led_task_, $decl.name)(
            outer_static: &'static $crate::led::LedOuterStatic<{ $upper($decl.name, _MAX_STEPS) }>,
            mut pin: $crate::esp_hal::gpio::Output<'static>,
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
}
