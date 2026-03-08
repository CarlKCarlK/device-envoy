//! A device abstraction for a single digital LED with animation support.
//!
//! This module provides a simple interface for controlling a single GPIO-connected LED
//! with support for on/off control and animated blinking sequences.
//!
//! See [`LedRp`] for the primary example and usage.

use core::borrow::Borrow;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use heapless::Vec;

pub use device_envoy_core::led::{Led, LedLevel, OnLevel};

use crate::{Error, Result};

/// Maximum number of animation frames allowed.
const MAX_FRAMES: usize = 32;

#[derive(Clone)]
pub(crate) enum LedCommand {
    /// Set LED level immediately.
    Set(LedLevel),
    /// Play an animation sequence (looping).
    Animate(Vec<(LedLevel, Duration), MAX_FRAMES>),
}

/// A device abstraction for a single digital LED with animation support.
///
/// # Hardware Requirements
///
/// This device requires a single GPIO pin connected to an LED. The LED can be wired
/// for either active-high (default) or active-low operation. The device supports both
/// polarities and controls the pin internally.
///
/// **Active-high wiring (default):** LED anode (long leg) -> 220 ohm resistor -> GPIO pin, LED cathode (short leg) -> GND
/// **Active-low wiring:** LED anode (long leg) -> 3.3V, LED cathode (short leg) -> 220 ohm resistor -> GPIO pin
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy_rp::{Result, led::{Led as _, LedLevel, LedRp, LedRpStatic, OnLevel}};
/// use embassy_time::Duration;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(p: embassy_rp::Peripherals, spawner: embassy_executor::Spawner) -> Result<()> {
///     static LED_RP_STATIC: LedRpStatic = LedRp::new_static();
///     let led_rp = LedRp::new(&LED_RP_STATIC, p.PIN_1, OnLevel::High, spawner)?;
///
///     // Turn the LED on
///     led_rp.set_level(LedLevel::On);
///     embassy_time::Timer::after(Duration::from_secs(1)).await;
///
///     // Turn the LED off
///     led_rp.set_level(LedLevel::Off);
///     embassy_time::Timer::after(Duration::from_millis(500)).await;
///
///     // Play a blinking animation (looping: 200ms on, 200ms off)
///     led_rp.animate([
///         (LedLevel::On, Duration::from_millis(200)),
///         (LedLevel::Off, Duration::from_millis(200)),
///     ]);
///
///     core::future::pending().await // run forever
/// }
/// ```
///
/// The device runs a background task that handles state transitions and animations.
/// Create the device once with [`LedRp::new`] and use the returned handle for all updates.
pub struct LedRp<'a>(&'a LedOuterStatic);

/// Signal for sending LED commands to the [`LedRp`] device.
pub(crate) type LedOuterStatic = Signal<CriticalSectionRawMutex, LedCommand>;

/// Static resources for the [`LedRp`] device.
pub struct LedRpStatic {
    outer: LedOuterStatic,
}

impl LedRpStatic {
    /// Creates static resources for a single LED device.
    pub(crate) const fn new() -> Self {
        Self {
            outer: Signal::new(),
        }
    }
}

impl LedRp<'_> {
    /// Creates a single LED device and spawns its background task; see [`LedRp`] docs.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new<P: embassy_rp::gpio::Pin>(
        led_rp_static: &'static LedRpStatic,
        pin: Peri<'static, P>,
        on_level: OnLevel,
        spawner: Spawner,
    ) -> Result<Self> {
        let pin_output = Output::new(pin, Level::Low);
        let token = device_loop(&led_rp_static.outer, pin_output, on_level);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(&led_rp_static.outer))
    }

    /// Creates static resources for [`LedRp::new`]; see [`LedRp`] docs.
    #[must_use]
    pub const fn new_static() -> LedRpStatic {
        LedRpStatic::new()
    }
}

impl Led for LedRp<'_> {
    /// Set the LED level immediately, replacing any running animation.
    ///
    /// See [LedRp struct example](LedRp) for usage.
    fn set_level(&self, led_level: LedLevel) {
        self.0.signal(LedCommand::Set(led_level));
    }

    /// Play a looped animation sequence of LED levels with durations.
    ///
    /// Accepts any iterator yielding (LedLevel, Duration) pairs or references, up to 32 frames.
    /// The animation will loop continuously until replaced by another command.
    /// This uses [`embassy_time::Duration`] for frame timing.
    /// See [LedRp struct example](LedRp) for usage.
    fn animate<I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(LedLevel, embassy_time::Duration)>,
    {
        let mut animation: Vec<(LedLevel, embassy_time::Duration), MAX_FRAMES> = Vec::new();
        for frame in frames {
            let frame = *frame.borrow();
            animation
                .push(frame)
                .expect("LED animation fits within MAX_FRAMES");
        }
        self.0.signal(LedCommand::Animate(animation));
    }
}

#[embassy_executor::task]
async fn device_loop(
    outer_static: &'static LedOuterStatic,
    mut pin: Output<'static>,
    on_level: OnLevel,
) -> ! {
    let mut command = LedCommand::Set(LedLevel::Off);
    set_pin_for_led_level(LedLevel::Off, &mut pin, on_level);

    loop {
        command = match command {
            LedCommand::Set(led_level) => {
                run_set_level_loop(led_level, outer_static, &mut pin, on_level).await
            }
            LedCommand::Animate(animation) => {
                run_animation_loop(animation, outer_static, &mut pin, on_level).await
            }
        };
    }
}

/// Set the physical pin state based on desired LED level and on_level.
fn set_pin_for_led_level(led_level: LedLevel, pin: &mut Output<'_>, on_level: OnLevel) {
    let pin_level = match (led_level, on_level) {
        (LedLevel::On, OnLevel::High) | (LedLevel::Off, OnLevel::Low) => Level::High,
        (LedLevel::Off, OnLevel::High) | (LedLevel::On, OnLevel::Low) => Level::Low,
    };
    pin.set_level(pin_level);
}

async fn run_set_level_loop(
    led_level: LedLevel,
    outer_static: &'static LedOuterStatic,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand {
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

async fn run_animation_loop(
    animation: Vec<(LedLevel, Duration), MAX_FRAMES>,
    outer_static: &'static LedOuterStatic,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand {
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
