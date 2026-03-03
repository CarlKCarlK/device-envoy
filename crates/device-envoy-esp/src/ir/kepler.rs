//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKepler`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use embassy_executor::Spawner;

use crate::ir::mapping::IrMapping;
#[cfg(target_os = "none")]
use crate::Result;
pub use device_envoy_core::ir::kepler::{IrKeplerStatic, KeplerButton};

type IrKeplerMapping<'a> = IrMapping<'a, KeplerButton, 21>;

/// A device abstraction for the SunFounder Kepler Kit IR remote.
pub struct IrKepler<'a> {
    mapping: IrKeplerMapping<'a>,
}

impl<'a> IrKepler<'a> {
    /// Create static channel resources for IR events.
    #[must_use]
    pub const fn new_static() -> IrKeplerStatic {
        IrKeplerStatic::new()
    }

    /// Create a new Kepler remote handler.
    ///
    /// # Errors
    /// Returns an error if the IR receiver cannot be initialized or the background task cannot be spawned.
    #[cfg(target_os = "none")]
    pub fn new(
        ir_kepler_static: &'static IrKeplerStatic,
        pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        channel_creator: impl esp_hal::rmt::RxChannelCreator<'static, esp_hal::Async>,
        spawner: Spawner,
    ) -> Result<Self> {
        use device_envoy_core::ir::kepler::KEPLER_MAPPING;
        let mapping = IrMapping::new(
            ir_kepler_static.inner(),
            pin,
            channel_creator,
            &KEPLER_MAPPING,
            spawner,
        )?;
        Ok(Self { mapping })
    }

    /// Wait for the next button press.
    ///
    /// Ignores button presses that are not recognized by the Kepler remote.
    pub async fn wait_for_press(&self) -> KeplerButton {
        self.mapping.wait_for_press().await
    }
}
