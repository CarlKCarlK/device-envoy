//! A device abstraction for a 4-digit, 7-segment LED display.
//!
//! See [`Led4`] for text, blinking, and animation control on ESP32.

pub use device_envoy_core::led4::{
    circular_outline_animation, AnimationFrame, BlinkState, ANIMATION_MAX_FRAMES, CELL_COUNT,
    SEGMENT_COUNT,
};

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

#[embassy_executor::task]
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

/// A device abstraction for a 4-digit, 7-segment LED display with blinking.
#[cfg(target_os = "none")]
pub struct Led4<'a>(&'a Led4OuterStatic);

#[cfg(target_os = "none")]
type Led4OuterStatic = device_envoy_core::led4::Led4CommandSignal;

/// Static for the [`Led4`] device.
#[cfg(target_os = "none")]
pub struct Led4Static {
    outer: Led4OuterStatic,
    display: Led4SimpleStatic,
}

#[cfg(target_os = "none")]
impl Led4Static {
    const fn new() -> Self {
        Self {
            outer: device_envoy_core::led4::Led4CommandSignal::new(),
            display: Led4Simple::new_static(),
        }
    }

    fn split(&self) -> (&Led4OuterStatic, &Led4SimpleStatic) {
        (&self.outer, &self.display)
    }
}

#[cfg(target_os = "none")]
impl Led4<'_> {
    /// Creates the display device and spawns its background task.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led4_static: &'static Led4Static,
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

    /// Creates static channel resources for [`Led4::new`].
    #[must_use]
    pub const fn new_static() -> Led4Static {
        Led4Static::new()
    }

    /// Sends text to the display with optional blinking.
    pub fn write_text(&self, text: [char; CELL_COUNT], blink_state: BlinkState) {
        signal_text(self.0, text, blink_state);
    }

    /// Plays a looped text animation using the provided frames.
    pub fn animate_text<I>(&self, animation: I)
    where
        I: IntoIterator,
        I::Item: core::borrow::Borrow<AnimationFrame>,
    {
        signal_animation(self.0, animation);
    }
}

#[embassy_executor::task]
#[cfg(target_os = "none")]
async fn led4_device_loop(
    outer_static: &'static Led4OuterStatic,
    display: Led4Simple<'static>,
) -> ! {
    run_command_loop(outer_static, |text| display.write_text(text)).await
}
