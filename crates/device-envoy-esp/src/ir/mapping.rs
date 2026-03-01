//! A device abstraction for mapping IR remote buttons to application-specific actions.
//!
//! See [`IrMapping`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use embassy_executor::Spawner;
use heapless::LinearMap;

use crate::ir::{Ir, IrEvent, IrStatic};
#[cfg(target_os = "none")]
use crate::Result;

/// Static channel for IR mapping events.
pub struct IrMappingStatic(IrStatic);

impl IrMappingStatic {
    /// Create static mapping resources.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self(Ir::new_static())
    }

    #[must_use]
    pub(crate) const fn inner(&self) -> &IrStatic {
        &self.0
    }
}

/// A generic device abstraction that maps IR remote button presses to user-defined button types.
pub struct IrMapping<'a, B, const N: usize> {
    ir: Ir<'a>,
    button_map: LinearMap<(u16, u8), B, N>,
}

impl<'a, B, const N: usize> IrMapping<'a, B, N>
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
        let ir = Ir::new(ir_mapping_static.inner(), pin, channel_creator, spawner)?;

        let mut linear_map = LinearMap::new();
        for &(addr, cmd, button) in button_map {
            let _ = linear_map.insert((addr, cmd), button);
        }
        // TODO0 Return an explicit error when mapping entries exceed LinearMap capacity.

        Ok(Self {
            ir,
            button_map: linear_map,
        })
    }

    /// Wait for the next recognized button press.
    ///
    /// Ignores button presses that are not in the button map.
    pub async fn wait_for_press(&self) -> B {
        loop {
            let IrEvent::Press { addr, cmd } = self.ir.wait_for_press().await;
            if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                return button;
            }
        }
    }
}
