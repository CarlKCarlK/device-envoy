//! Platform-independent static resources for IR button mapping.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::IrStatic;

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
