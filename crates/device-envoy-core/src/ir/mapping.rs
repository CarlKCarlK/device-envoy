//! Platform-independent static resources for IR button mapping.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::IrStatic;
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

impl<T, Button> IrMapping<Button> for &T
where
    T: IrMapping<Button> + ?Sized,
{
    async fn wait_for_press(&self) -> Button {
        (*self).wait_for_press().await
    }
}

/// Static channel resources for IR mapping events.
///
/// Create with `IrMapping::new_static()` from the platform-specific crate.
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
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

impl Default for IrMappingStatic {
    fn default() -> Self {
        Self::new()
    }
}
