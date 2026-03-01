//! A device abstraction for a single digital LED with animation support.
//!
//! This module provides a simple interface for controlling a single GPIO-connected LED
//! with support for on/off control and animated blinking sequences.
//!
//! See [`Led`] for the primary example and usage.

use core::borrow::Borrow;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use heapless::Vec;

use crate::{Error, Result};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of animation frames allowed.
const MAX_FRAMES: usize = 32;

// ============================================================================
// OnLevel - What pin level turns the LED on
// ============================================================================

/// What pin level turns the LED on (depends on wiring).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, defmt::Format, Default)]
pub enum OnLevel {
    /// LED lights when pin is HIGH (standard wiring).
    /// LED anode → 220Ω resistor → GPIO pin, LED cathode → GND
    #[default]
    High,

    /// LED lights when pin is LOW (alternative wiring).
    /// LED anode → 3.3V, LED cathode → 220Ω resistor → GPIO pin
    Low,
}

// ============================================================================
// LedCommand Enum
// ============================================================================

#[derive(Clone)]
pub(crate) enum LedCommand {
    /// Set LED level immediately.
    Set(Level),
    /// Play an animation sequence (looping).
    Animate(Vec<(Level, Duration), MAX_FRAMES>),
}

// ============================================================================
// Led Virtual Device
// ============================================================================

/// A device abstraction for a single digital LED with animation support.
///
/// # Hardware Requirements
///
/// This device requires a single GPIO pin connected to an LED. The LED can be wired
/// for either active-high (default) or active-low operation. The device supports both
/// polarities and controls the pin internally.
///
/// **Active-high wiring (default):** LED anode (long leg) → 220Ω resistor → GPIO pin, LED cathode (short leg) → GND
/// **Active-low wiring:** LED anode (long leg) → 3.3V, LED cathode (short leg) → 220Ω resistor → GPIO pin
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// use device_envoy::{Result, led::{Led, LedStatic, OnLevel}};
/// use embassy_time::Duration;
/// use embassy_rp::gpio::Level;
/// # #[panic_handler]
/// # fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
///
/// async fn example(p: embassy_rp::Peripherals, spawner: embassy_executor::Spawner) -> Result<()> {
///     static LED_STATIC: LedStatic = Led::new_static();
///     let led = Led::new(&LED_STATIC, p.PIN_1, OnLevel::High, spawner)?;
///
///     // Turn the LED on
///     led.set_level(Level::High);
///     embassy_time::Timer::after(Duration::from_secs(1)).await;
///
///     // Turn the LED off
///     led.set_level(Level::Low);
///     embassy_time::Timer::after(Duration::from_millis(500)).await;
///
///     // Play a blinking animation (looping: 200ms on, 200ms off)
///     led.animate(&[(Level::High, Duration::from_millis(200)), (Level::Low, Duration::from_millis(200))]);
///
///     core::future::pending().await // run forever
/// }
/// ```
///
/// The device runs a background task that handles state transitions and animations.
/// Create the device once with [`Led::new`] and use the returned handle for all updates.
pub struct Led<'a>(&'a LedOuterStatic);

/// Signal for sending LED commands to the [`Led`] device.
pub(crate) type LedOuterStatic = Signal<CriticalSectionRawMutex, LedCommand>;

/// Static resources for the [`Led`] device.
pub struct LedStatic {
    outer: LedOuterStatic,
}

impl LedStatic {
    /// Creates static resources for a single LED device.
    pub(crate) const fn new() -> Self {
        Self {
            outer: Signal::new(),
        }
    }
}

impl Led<'_> {
    /// Creates a single LED device and spawns its background task; see [`Led`] docs.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new<P: embassy_rp::gpio::Pin>(
        led_static: &'static LedStatic,
        pin: Peri<'static, P>,
        on_level: OnLevel,
        spawner: Spawner,
    ) -> Result<Self> {
        let pin_output = Output::new(pin, Level::Low);
        let token = device_loop(&led_static.outer, pin_output, on_level);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(&led_static.outer))
    }

    /// Creates static resources for [`Led::new`]; see [`Led`] docs.
    #[must_use]
    pub const fn new_static() -> LedStatic {
        LedStatic::new()
    }

    /// Set the LED level immediately, replacing any running animation.
    ///
    /// See [Led struct example](Self) for usage.
    pub fn set_level(&self, level: Level) {
        self.0.signal(LedCommand::Set(level));
    }

    /// Play a looped animation sequence of LED levels with durations.
    ///
    /// Accepts any iterator yielding (Level, Duration) pairs or references, up to 32 frames.
    /// The animation will loop continuously until replaced by another command.
    /// This uses [`embassy_time::Duration`] for frame timing.
    /// See [Led struct example](Self) for usage.
    pub fn animate<I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(Level, embassy_time::Duration)>,
    {
        let mut animation: Vec<(Level, embassy_time::Duration), MAX_FRAMES> = Vec::new();
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
    let mut command = LedCommand::Set(Level::Low);
    set_pin_for_led_level(Level::Low, &mut pin, on_level);

    loop {
        command = match command {
            LedCommand::Set(level) => {
                run_set_level_loop(level, outer_static, &mut pin, on_level).await
            }
            LedCommand::Animate(animation) => {
                run_animation_loop(animation, outer_static, &mut pin, on_level).await
            }
        };
    }
}

/// Set the physical pin state based on desired LED level and on_level.
fn set_pin_for_led_level(led_level: Level, pin: &mut Output<'_>, on_level: OnLevel) {
    let pin_level = match (led_level, on_level) {
        (Level::High, OnLevel::High) | (Level::Low, OnLevel::Low) => Level::High,
        (Level::Low, OnLevel::High) | (Level::High, OnLevel::Low) => Level::Low,
    };
    pin.set_level(pin_level);
}

async fn run_set_level_loop(
    level: Level,
    outer_static: &'static LedOuterStatic,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand {
    set_pin_for_led_level(level, pin, on_level);

    loop {
        match outer_static.wait().await {
            LedCommand::Set(new_level) => {
                if new_level == level {
                    // No change, keep waiting
                    continue;
                } else {
                    return LedCommand::Set(new_level);
                }
            }
            other => return other,
        }
    }
}

async fn run_animation_loop(
    animation: Vec<(Level, Duration), MAX_FRAMES>,
    outer_static: &'static LedOuterStatic,
    pin: &mut Output<'_>,
    on_level: OnLevel,
) -> LedCommand {
    if animation.is_empty() {
        return LedCommand::Animate(animation);
    }

    let mut frame_index = 0;

    loop {
        let (level, duration) = animation[frame_index];

        set_pin_for_led_level(level, pin, on_level);

        frame_index = (frame_index + 1) % animation.len();

        // Wait for duration, but check for new commands
        match embassy_futures::select::select(Timer::after(duration), outer_static.wait()).await {
            embassy_futures::select::Either::First(_) => {
                // Duration elapsed, continue animation
            }
            embassy_futures::select::Either::Second(command) => {
                // New command received
                return command;
            }
        }
    }
}
