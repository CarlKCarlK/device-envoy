//! Platform-independent static resources for IR button mapping.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::{Ir, IrEvent, IrStatic};
use heapless::LinearMap;

/// Platform-agnostic IR button mapper device contract.
///
/// Platform crates implement this for their concrete `IrMapping` types so shared logic can wait
/// for mapped button presses without depending on platform-specific modules.
///
/// This trait is intended for app-level button enums mapped from `(addr, cmd)` pairs
/// in platform-specific constructors.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::ir::IrMapping;
///
/// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// enum RemoteKeys {
///     Power,
///     Play,
///     Stop,
/// }
///
/// async fn handle_mapped_button_presses(ir_mapping: &impl IrMapping<RemoteKeys>) -> ! {
///     loop {
///         let remote_key = ir_mapping.wait_for_press().await;
///         // Use mapped key in app logic.
///         match remote_key {
///             RemoteKeys::Power => {
///                 // Handle power.
///             }
///             RemoteKeys::Play => {
///                 // Handle play.
///             }
///             RemoteKeys::Stop => {
///                 // Handle stop.
///             }
///         }
///     }
/// }
///
/// # struct DemoIrMapping;
/// # impl IrMapping<RemoteKeys> for DemoIrMapping {
/// #     async fn wait_for_press(&self) -> RemoteKeys {
/// #         RemoteKeys::Power
/// #     }
/// # }
/// # fn main() {
/// #     let ir_mapping = DemoIrMapping;
/// #     let _future = handle_mapped_button_presses(&ir_mapping);
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait IrMapping<Button> {
    /// Wait for the next recognized mapped button press.
    ///
    /// See the [IrMapping trait documentation](Self) for usage examples.
    async fn wait_for_press(&self) -> Button;
}

/// Shared mapper implementation for platform IR devices.
///
/// This owns a platform IR device plus a `(addr, cmd) -> Button` table and provides
/// the common `wait_for_press` behavior used by RP/ESP wrappers.
pub struct IrMappingAdapter<I, B, const N: usize> {
    ir: I,
    button_map: LinearMap<(u16, u8), B, N>,
}

impl<I, B, const N: usize> IrMappingAdapter<I, B, N>
where
    B: Copy,
{
    /// Build a new adapter from a platform IR device and mapping table entries.
    ///
    /// Panics if `button_map` exceeds `N` entries or contains duplicate `(addr, cmd)` keys.
    #[must_use]
    pub fn new(ir: I, button_map: &[(u16, u8, B)]) -> Self {
        let mut linear_map = LinearMap::new();
        for &(addr, cmd, button) in button_map {
            let previous_button = match linear_map.insert((addr, cmd), button) {
                Ok(previous_button) => previous_button,
                Err(_) => panic!("button_map entries exceed IrMapping capacity"),
            };
            assert!(
                previous_button.is_none(),
                "button_map contains duplicate (addr, cmd) entries"
            );
        }

        Self {
            ir,
            button_map: linear_map,
        }
    }
}

impl<I, B, const N: usize> IrMapping<B> for IrMappingAdapter<I, B, N>
where
    I: Ir,
    B: Copy,
{
    async fn wait_for_press(&self) -> B {
        loop {
            let IrEvent::Press { addr, cmd } = self.ir.wait_for_press().await;
            #[cfg(feature = "defmt")]
            defmt::info!("IR received - addr=0x{:04X} cmd=0x{:02X}", addr, cmd);
            if let Some(&button) = self.button_map.get(&(addr, cmd)) {
                return button;
            }
            #[cfg(feature = "defmt")]
            defmt::info!("  (unrecognized - ignoring)");
        }
    }
}

/// Static channel resources for IR mapping events.
///
/// Create with `IrMapping::new_static()` from the platform-specific crate.
pub struct IrMappingStatic(IrStatic);

impl IrMappingStatic {
    /// Create static mapping resources.
    #[must_use]
    pub const fn new() -> Self {
        Self(IrStatic::new())
    }

    /// Get a reference to the inner static IR resources.
    #[must_use]
    pub const fn inner(&self) -> &IrStatic {
        &self.0
    }
}
