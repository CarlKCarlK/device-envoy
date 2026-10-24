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
// Must be `pub` for macro expansion at downstream call sites.
#[doc(hidden)]
pub use device_envoy_core::led::{
    DEFAULT_MAX_STEPS, LedCommand, LedOuterStatic, LedStatic, run_animation_loop,
    run_set_level_loop, set_pin_for_led_level,
};
pub mod led_generated;

macro_schema::define! {
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
        static $upper($decl.name, _STATIC): $crate::led::LedStatic<{ $upper($decl.name, _MAX_STEPS) }> =
            $crate::led::LedStatic::new();

        #[cfg(target_os = "none")]
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name(&'static $crate::led::LedOuterStatic<{ $upper($decl.name, _MAX_STEPS) }>);

        #[cfg(target_os = "none")]
        impl $decl.name {
            /// Maximum number of steps in one animation.
            pub const MAX_STEPS: usize = $upper($decl.name, _MAX_STEPS);

            /// Creates the LED and spawns its background task.
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
                let mut animation: $crate::__heapless::Vec<
                    ($crate::led::LedLevel, embassy_time::Duration),
                    { $upper($decl.name, _MAX_STEPS) },
                > = $crate::__heapless::Vec::new();
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
