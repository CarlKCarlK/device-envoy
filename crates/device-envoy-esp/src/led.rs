//! A device abstraction for a single digital LED with animation support.
//!
//! See [`LedEsp`] for constructors and [`Led`] for trait methods.

pub use device_envoy_core::led::{Led, LedLevel, OnLevel};

#[cfg(target_os = "none")]
use core::borrow::Borrow;
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
#[cfg(target_os = "none")]
use embassy_time::{Duration, Timer};
#[cfg(target_os = "none")]
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};
#[cfg(target_os = "none")]
use heapless::Vec;

#[cfg(target_os = "none")]
use crate::{Error, Result};

#[cfg(target_os = "none")]
const MAX_FRAMES: usize = 32;

#[cfg(target_os = "none")]
#[derive(Clone)]
enum LedCommand {
    Set(LedLevel),
    Animate(Vec<(LedLevel, Duration), MAX_FRAMES>),
}

#[cfg(target_os = "none")]
type LedOuterStatic = Signal<CriticalSectionRawMutex, LedCommand>;

/// Static resources for a [`LedEsp`] device.
#[cfg(target_os = "none")]
pub struct LedEspStatic {
    outer: LedOuterStatic,
}

#[cfg(target_os = "none")]
impl LedEspStatic {
    const fn new() -> Self {
        Self {
            outer: Signal::new(),
        }
    }
}

/// ESP implementation of the single LED device abstraction.
#[cfg(target_os = "none")]
pub struct LedEsp<'a>(&'a LedOuterStatic);

#[cfg(target_os = "none")]
impl LedEsp<'_> {
    /// Create a single LED device and spawn its background task.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led_esp_static: &'static LedEspStatic,
        pin: impl OutputPin + 'static,
        on_level: OnLevel,
        spawner: Spawner,
    ) -> Result<Self> {
        let pin_output = Output::new(pin, Level::Low, OutputConfig::default());
        let token = led_device_loop(&led_esp_static.outer, pin_output, on_level);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(&led_esp_static.outer))
    }

    /// Create static resources for [`LedEsp::new`].
    #[must_use]
    pub const fn new_static() -> LedEspStatic {
        LedEspStatic::new()
    }
}

#[cfg(target_os = "none")]
impl Led for LedEsp<'_> {
    fn set_level(&self, led_level: LedLevel) {
        self.0.signal(LedCommand::Set(led_level));
    }

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

#[cfg(target_os = "none")]
#[embassy_executor::task]
async fn led_device_loop(
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

#[cfg(target_os = "none")]
fn set_pin_for_led_level(led_level: LedLevel, pin: &mut Output<'_>, on_level: OnLevel) {
    let pin_level = match (led_level, on_level) {
        (LedLevel::On, OnLevel::High) | (LedLevel::Off, OnLevel::Low) => Level::High,
        (LedLevel::Off, OnLevel::High) | (LedLevel::On, OnLevel::Low) => Level::Low,
    };
    pin.set_level(pin_level);
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
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
            embassy_futures::select::Either::Second(command) => return command,
        }
    }
}
