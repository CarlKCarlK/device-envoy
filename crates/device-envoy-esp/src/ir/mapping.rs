//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMappingEsp`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use embassy_executor::Spawner;

use crate::ir::{IrEsp, IrMapping, IrMappingAdapter};
#[cfg(target_os = "none")]
use crate::Result;
pub use device_envoy_core::ir::mapping::IrMappingStatic;

/// A generic device abstraction that maps IR remote button presses to user-defined button types.
pub struct IrMappingEsp<'a, B, const N: usize> {
    mapping: IrMappingAdapter<IrEsp<'a>, B, N>,
}

impl<'a, B, const N: usize> IrMappingEsp<'a, B, N>
where
    B: Copy,
{
    /// Create static channel resources for IR mapping events.
    #[must_use]
    pub const fn new_static() -> IrMappingStatic {
        IrMappingStatic::new()
    }

    /// Create a new IR remote button mapper.
    ///
    /// # Errors
    /// Returns an error if the IR receiver cannot be initialized or the background task cannot be spawned.
    #[cfg(target_os = "none")]
    pub fn new(
        ir_mapping_static: &'static IrMappingStatic,
        pin: impl esp_hal::gpio::interconnect::PeripheralInput<'static>,
        channel_creator: impl esp_hal::rmt::RxChannelCreator<'static, esp_hal::Async>,
        button_map: &[(u16, u8, B)],
        spawner: Spawner,
    ) -> Result<Self> {
        let ir = IrEsp::new(ir_mapping_static.inner(), pin, channel_creator, spawner)?;
        let mapping = IrMappingAdapter::new(ir, button_map);
        Ok(Self { mapping })
    }
}

impl<B, const N: usize> IrMapping<B> for IrMappingEsp<'_, B, N>
where
    B: Copy,
{
    async fn wait_for_press(&self) -> B {
        self.mapping.wait_for_press().await
    }
}
