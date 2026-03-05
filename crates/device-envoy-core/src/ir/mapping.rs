//! Platform-independent static resources for IR button mapping.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::IrStatic;

/// Platform-agnostic IR button mapper device contract.
///
/// Platform crates implement this for their concrete `IrMapping` types so shared logic can wait
/// for mapped button presses without depending on platform-specific modules.
#[allow(async_fn_in_trait)]
pub trait IrMapping<Button> {
    /// Wait for the next recognized mapped button press.
    async fn wait_for_press(&self) -> Button;
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
