//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKeplerEsp`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

use device_envoy_core::ir::{IrKepler, IrMapping as _};
#[cfg(target_os = "none")]
use embassy_executor::Spawner;

use crate::ir::mapping::IrMappingEsp;
#[cfg(target_os = "none")]
use crate::Result;
pub use device_envoy_core::ir::kepler::{IrKeplerStatic, KeplerButton};

type IrKeplerMapping<'a> = IrMappingEsp<'a, KeplerButton, 21>;

/// A device abstraction for the SunFounder Kepler Kit IR remote.
pub struct IrKeplerEsp<'a> {
    mapping: IrKeplerMapping<'a>,
}

impl<'a> IrKeplerEsp<'a> {
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
        let mapping = IrMappingEsp::new(
            ir_kepler_static.inner(),
            pin,
            channel_creator,
            &KEPLER_MAPPING,
            spawner,
        )?;
        Ok(Self { mapping })
    }

}

impl IrKepler for IrKeplerEsp<'_> {
    async fn wait_for_press(&self) -> KeplerButton {
        self.mapping.wait_for_press().await
    }
}
