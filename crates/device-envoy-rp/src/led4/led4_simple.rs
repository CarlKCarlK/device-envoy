//! A device abstraction for a non-blinking 4-digit 7-segment LED display.
//!
//! See [`Led4Simple`] for usage.

use core::convert::Infallible;

use super::OutputArray;
use super::{CELL_COUNT, SEGMENT_COUNT};
use crate::Error;
use crate::Result;
#[cfg(feature = "display-trace")]
use defmt::info;
use device_envoy_core::led4::{
    BitMatrixLed4, Led4OutputAdapter, Led4SimpleLoopError, run_simple_loop,
};
use embassy_executor::{SpawnError, Spawner};
use embassy_rp::gpio::Level;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

/// Static for the [`Led4Simple`] device.
pub struct Led4SimpleStatic(Signal<CriticalSectionRawMutex, BitMatrixLed4>);

impl Led4SimpleStatic {
    pub const fn new() -> Self {
        Self(Signal::new())
    }

    fn signal(&self, bit_matrix: BitMatrixLed4) {
        self.0.signal(bit_matrix);
    }
}

/// A device abstraction for a non-blinking 4-digit 7-segment LED display.
///
/// Use this if you don't need animation or blinking. For blinking or animation support, use [`Led4Rp`](crate::led4::Led4Rp) instead.
///
/// This is an internal struct. Users should use [`Led4Rp`](crate::led4::Led4Rp) instead.
pub struct Led4Simple<'a>(&'a Led4SimpleStatic);

impl Led4Simple<'_> {
    /// Creates static channel resources for the display.
    #[must_use]
    pub const fn new_static() -> Led4SimpleStatic {
        Led4SimpleStatic::new()
    }

    /// Creates the display device and spawns its background task.
    ///
    /// # Errors
    ///
    /// Returns an error if the task cannot be spawned.
    #[must_use = "Must be used to manage the spawned task"]
    pub fn new(
        led4_simple_static: &'static Led4SimpleStatic,
        cell_pins: OutputArray<'static, CELL_COUNT>,
        segment_pins: OutputArray<'static, SEGMENT_COUNT>,
        spawner: Spawner,
    ) -> Result<Self, SpawnError> {
        let token = device_loop(cell_pins, segment_pins, led4_simple_static);
        spawner.spawn(token)?;
        Ok(Self(led4_simple_static))
    }

    /// Sends text to the display.
    pub fn write_text(&self, text: [char; CELL_COUNT]) {
        #[cfg(feature = "display-trace")]
        info!("write_chars: {:?}", text);
        self.0.signal(BitMatrixLed4::from_text(&text));
    }
}

#[embassy_executor::task(pool_size = 2)]
pub(crate) async fn device_loop(
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
    led4_simple_static: &'static Led4SimpleStatic,
) -> ! {
    let err = inner_device_loop(cell_pins, segment_pins, led4_simple_static)
        .await
        .unwrap_err();
    panic!("{err}");
}

async fn inner_device_loop(
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
    led4_simple_static: &'static Led4SimpleStatic,
) -> Result<Infallible> {
    let mut rp_led4_output = RpLed4Output {
        cell_pins,
        segment_pins,
    };
    run_simple_loop(&mut rp_led4_output, &led4_simple_static.0)
        .await
        .map_err(Error::from)
}

struct RpLed4Output {
    cell_pins: OutputArray<'static, CELL_COUNT>,
    segment_pins: OutputArray<'static, SEGMENT_COUNT>,
}

impl Led4OutputAdapter for RpLed4Output {
    type Error = Error;

    fn set_segments_from_nonzero_bits(&mut self, bits: core::num::NonZeroU8) {
        self.segment_pins.set_from_nonzero_bits(bits);
    }

    fn set_cells_active(&mut self, indexes: &[u8], active: bool) -> Result<(), Self::Error> {
        let level = if active { Level::Low } else { Level::High };
        self.cell_pins.set_levels_at_indexes(indexes, level)
    }
}

impl From<Led4SimpleLoopError<Error>> for Error {
    fn from(error: Led4SimpleLoopError<Error>) -> Self {
        match error {
            Led4SimpleLoopError::BitsToIndexes(error) => Self::from(error),
            Led4SimpleLoopError::Output(error) => error,
        }
    }
}
