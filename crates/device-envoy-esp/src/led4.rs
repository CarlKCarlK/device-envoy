//! A device abstraction for a 4-digit, 7-segment LED display for text with optional animation and blinking.
//!
//! See [`Led4Esp`] for the primary text/blinking example and [`Led4`] for trait methods.
//!
//! **Limations**: You can create up to two concurrent `Led4Esp` instances per program; a third is expected to fail at runtime because the `led4` task pool uses `pool_size = 2`. Animation APIs support up to 16 steps per animation (`ANIMATION_MAX_FRAMES`).
//!
//! This module provides device abstraction for controlling common-cathode
//! 4-digit 7-segment LED displays. Supports displaying text and numbers with
//! optional blinking.

pub use device_envoy_core::led4::{
    circular_outline_animation, AnimationFrame, BlinkState, Led4, ANIMATION_MAX_FRAMES,
};

const CELL_COUNT: usize = device_envoy_core::led4::CELL_COUNT;
const SEGMENT_COUNT: usize = device_envoy_core::led4::SEGMENT_COUNT;

#[cfg(target_os = "none")]
use core::convert::Infallible;

#[cfg(target_os = "none")]
use device_envoy_core::led4::{
    run_command_loop, run_simple_loop, signal_animation, signal_text, BitMatrixLed4,
    Led4OutputAdapter, Led4SimpleLoopError,
};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;
#[cfg(target_os = "none")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

#[cfg(target_os = "none")]
use crate::{Error, Result};

#[cfg(target_os = "none")]
mod output_array;
#[cfg(target_os = "none")]
pub use output_array::OutputArray;

#[cfg(target_os = "none")]
struct Led4SimpleStatic(Signal<CriticalSectionRawMutex, BitMatrixLed4>);

#[cfg(target_os = "none")]
impl Led4SimpleStatic {
    const fn new() -> Self {
        Self(Signal::new())
    }

    fn signal(&self, bit_matrix_led4: BitMatrixLed4) {
        self.0.signal(bit_matrix_led4);
    }
}

#[cfg(target_os = "none")]
struct Led4Simple<'a>(&'a Led4SimpleStatic);

#[cfg(target_os = "none")]
impl Led4Simple<'_> {
    const fn new_static() -> Led4SimpleStatic {
        Led4SimpleStatic::new()
    }

    #[must_use = "Must be used to manage the spawned task"]
    fn new(
        led4_simple_static: &'static Led4SimpleStatic,
        cell_pins: OutputArray<'static, CELL_COUNT>,
        segment_pins: OutputArray<'static, SEGMENT_COUNT>,
        spawner: Spawner,
    ) -> Result<Self> {
        let token = led4_simple_device_loop(cell_pins, segment_pins, led4_simple_static);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(led4_simple_static))
    }

    fn write_text(&self, text: [char; CELL_COUNT]) {
        self.0.signal(BitMatrixLed4::from_text(&text));
    }
}

#[embassy_executor::task(pool_size = 2)]
#[cfg(target_os = "none")]
async fn led4_simple_device_loop(
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
    led4_simple_static: &'static Led4SimpleStatic,
) -> ! {
    let error = inner_led4_simple_device_loop(cell_pins, segment_pins, led4_simple_static)
        .await
        .unwrap_err();
    panic!("{error:?}");
}

#[cfg(target_os = "none")]
async fn inner_led4_simple_device_loop(
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
    led4_simple_static: &'static Led4SimpleStatic,
) -> Result<Infallible> {
    let mut esp_led4_output = EspLed4Output {
        cell_pins,
        segment_pins,
    };
    run_simple_loop(&mut esp_led4_output, &led4_simple_static.0)
        .await
        .map_err(Error::from)
}

#[cfg(target_os = "none")]
struct EspLed4Output {
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
}

#[cfg(target_os = "none")]
impl Led4OutputAdapter for EspLed4Output {
    type Error = Error;

    fn set_segments_from_nonzero_bits(&mut self, bits: core::num::NonZeroU8) {
        self.segment_pins.set_from_nonzero_bits(bits);
    }

    fn set_cells_active(&mut self, indexes: &[u8], active: bool) -> Result<(), Self::Error> {
        let level = if active {
            esp_hal::gpio::Level::Low
        } else {
            esp_hal::gpio::Level::High
        };
        self.cell_pins.set_levels_at_indexes(indexes, level)
    }
}

#[cfg(target_os = "none")]
impl From<Led4SimpleLoopError<Error>> for Error {
    fn from(error: Led4SimpleLoopError<Error>) -> Self {
        match error {
            Led4SimpleLoopError::BitsToIndexes(error) => Self::from(error),
            Led4SimpleLoopError::Output(error) => error,
        }
    }
}

/// A device abstraction for a 4-digit, 7-segment LED display with blinking support.
///
/// # Hardware Requirements
///
/// This abstraction is designed for common-cathode 7-segment displays where:
/// - Cell pins control which digit is active (LOW = on, HIGH = off)
/// - Segment pins control which segments light up (HIGH = on, LOW = off)
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use esp_backtrace as _;
/// use device_envoy_esp::{
///     Error, Result, init_and_start,
///     led4::{BlinkState, Led4 as _, Led4Esp, Led4EspStatic, OutputArray, circular_outline_animation},
/// };
/// use esp_hal::gpio::{Level, Output, OutputConfig};
/// use embassy_time::{Duration, Timer};
///
/// # #[esp_rtos::main]
/// # async fn main(spawner: embassy_executor::Spawner) -> ! {
/// #     match example(spawner).await {
/// #         Ok(()) => loop {},
/// #         Err(error) => panic!("{error:?}"),
/// #     }
/// # }
/// async fn example(spawner: embassy_executor::Spawner) -> Result<(), Error> {
///     init_and_start!(p);
///
///     let cells = OutputArray::new([
///         Output::new(p.GPIO14, Level::High, OutputConfig::default()),
///         Output::new(p.GPIO13, Level::High, OutputConfig::default()),
///         Output::new(p.GPIO12, Level::High, OutputConfig::default()),
///         Output::new(p.GPIO11, Level::High, OutputConfig::default()),
///     ]);
///
///     let segments = OutputArray::new([
///         Output::new(p.GPIO10, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO9, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO4, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO3, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO8, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO18, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO17, Level::Low, OutputConfig::default()),
///         Output::new(p.GPIO16, Level::Low, OutputConfig::default()),
///     ]);
///
///     static LED4_STATIC: Led4EspStatic = Led4Esp::new_static();
///     let display = Led4Esp::new(&LED4_STATIC, cells, segments, spawner)?;
///
///     // Blink "1234" for three seconds.
///     display.write_text(['1', '2', '3', '4'], BlinkState::BlinkingAndOn);
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Run the circular outline animation for three seconds.
///     display.animate_text(circular_outline_animation(true));
///     Timer::after(Duration::from_secs(3)).await;
///
///     // Show "rUSt" solid forever.
///     display.write_text(['r', 'U', 'S', 't'], BlinkState::Solid);
///     core::future::pending().await
/// }
/// ```
///
/// Beyond simple text, the driver can loop animations via [`Led4::animate_text`].
/// The struct owns the background task and signal wiring; create it once with
/// [`Led4Esp::new`] and use the returned handle for all display updates.
#[cfg(target_os = "none")]
pub struct Led4Esp<'a>(&'a Led4EspOuterStatic);

#[cfg(target_os = "none")]
type Led4EspOuterStatic = device_envoy_core::led4::Led4CommandSignal;

/// Static for the [`Led4Esp`] device.
#[cfg(target_os = "none")]
pub struct Led4EspStatic {
    outer: Led4EspOuterStatic,
    display: Led4SimpleStatic,
}

#[cfg(target_os = "none")]
impl Led4EspStatic {
    const fn new() -> Self {
        Self {
            outer: device_envoy_core::led4::Led4CommandSignal::new(),
            display: Led4Simple::new_static(),
        }
    }

    fn split(&self) -> (&Led4EspOuterStatic, &Led4SimpleStatic) {
        (&self.outer, &self.display)
    }
}

#[cfg(target_os = "none")]
impl Led4Esp<'_> {
    /// Creates the display device and spawns its background task.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led4_static: &'static Led4EspStatic,
        cell_pins: OutputArray<'static, CELL_COUNT>,
        segment_pins: OutputArray<'static, SEGMENT_COUNT>,
        spawner: Spawner,
    ) -> Result<Self> {
        let (outer_static, display_static) = led4_static.split();
        let display = Led4Simple::new(display_static, cell_pins, segment_pins, spawner)?;
        let token = led4_device_loop(outer_static, display);
        spawner.spawn(token).map_err(Error::TaskSpawn)?;
        Ok(Self(outer_static))
    }

    /// Creates static channel resources for [`Led4Esp::new`].
    #[must_use]
    pub const fn new_static() -> Led4EspStatic {
        Led4EspStatic::new()
    }
}

#[cfg(target_os = "none")]
impl device_envoy_core::led4::Led4 for Led4Esp<'_> {
    fn write_text(&self, text: [char; CELL_COUNT], blink_state: BlinkState) {
        signal_text(self.0, text, blink_state);
    }

    fn animate_text<I>(&self, animation: I)
    where
        I: IntoIterator,
        I::Item: core::borrow::Borrow<AnimationFrame>,
    {
        signal_animation(self.0, animation);
    }
}

#[embassy_executor::task(pool_size = 2)]
#[cfg(target_os = "none")]
async fn led4_device_loop(
    outer_static: &'static Led4EspOuterStatic,
    display: Led4Simple<'static>,
) -> ! {
    run_command_loop(outer_static, |text| display.write_text(text)).await
}
