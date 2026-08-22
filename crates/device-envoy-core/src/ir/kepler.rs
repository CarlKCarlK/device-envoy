//! Platform-independent types for the SunFounder Kepler Kit IR remote control.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::mapping::IrMappingStatic;

/// Button types for the SunFounder Kepler Kit remote control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KeplerKeys {
    /// Power button.
    Power,
    /// Mode button.
    Mode,
    /// Mute button.
    Mute,
    /// Play/Pause button.
    PlayPause,
    /// Previous track button.
    Prev,
    /// Next track button.
    Next,
    /// Equalizer button.
    Eq,
    /// Minus/Decrease volume button.
    Minus,
    /// Plus/Increase volume button.
    Plus,
    /// Numbered button (0-9).
    Num(u8),
    /// Repeat button.
    Repeat,
    /// USB/SD card mode button.
    USd,
}

/// Platform-agnostic SunFounder Kepler IR device contract.
///
/// Platform crates implement this for their concrete `IrKepler` types so shared logic can wait
/// for button presses without depending on platform-specific modules.
///
/// This trait provides a simple interface with built-in Kepler button mappings.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::ir::{IrKepler, KeplerKeys};
///
/// async fn handle_kepler_button_presses(ir_kepler: &impl IrKepler) -> ! {
///     loop {
///         let kepler_button = ir_kepler.wait_for_press().await;
///         // Use mapped Kepler button in app logic.
///         match kepler_button {
///             KeplerKeys::Power => {
///                 // Handle power.
///             }
///             KeplerKeys::PlayPause => {
///                 // Handle play/pause.
///             }
///             _ => {
///                 // Handle all other Kepler buttons.
///             }
///         }
///     }
/// }
///
/// # struct DemoIrKepler;
/// # impl IrKepler for DemoIrKepler {
/// #     async fn wait_for_press(&self) -> KeplerKeys {
/// #         KeplerKeys::Power
/// #     }
/// # }
/// # fn main() {
/// #     let ir_kepler = DemoIrKepler;
/// #     let _future = handle_kepler_button_presses(&ir_kepler);
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait IrKepler {
    /// Wait for the next recognized Kepler button press.
    ///
    /// See the [IrKepler trait documentation](Self) for usage examples.
    async fn wait_for_press(&self) -> KeplerKeys;
}

impl<T> IrKepler for &T
where
    T: IrKepler + ?Sized,
{
    async fn wait_for_press(&self) -> KeplerKeys {
        (*self).wait_for_press().await
    }
}

/// Static resources for Kepler IR remote events.
///
/// See the platform-specific crate for usage examples.
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
pub struct IrKeplerStatic(IrMappingStatic);

impl IrKeplerStatic {
    /// Create static resources for the Kepler remote.
    #[must_use]
    pub const fn new() -> Self {
        Self(IrMappingStatic::new())
    }

    /// Get a reference to the inner mapping static resources.
    #[must_use]
    pub const fn inner(&self) -> &IrMappingStatic {
        &self.0
    }
}

impl Default for IrKeplerStatic {
    fn default() -> Self {
        Self::new()
    }
}

/// Button mapping for the SunFounder Kepler Kit remote (ordered to match physical layout).
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
pub const KEPLER_MAPPING: [(u16, u8, KeplerKeys); 21] = [
    // Row 1: Power, Mode, Mute
    (0x0000, 0x45, KeplerKeys::Power),
    (0x0000, 0x46, KeplerKeys::Mode),
    (0x0000, 0x47, KeplerKeys::Mute),
    // Row 2: PlayPause, Prev, Next
    (0x0000, 0x44, KeplerKeys::PlayPause),
    (0x0000, 0x40, KeplerKeys::Prev),
    (0x0000, 0x43, KeplerKeys::Next),
    // Row 3: EQ, Minus, Plus
    (0x0000, 0x07, KeplerKeys::Eq),
    (0x0000, 0x15, KeplerKeys::Minus),
    (0x0000, 0x09, KeplerKeys::Plus),
    // Row 4: 0, Repeat, U/SD
    (0x0000, 0x16, KeplerKeys::Num(0)),
    (0x0000, 0x19, KeplerKeys::Repeat),
    (0x0000, 0x0D, KeplerKeys::USd),
    // Row 5: 1, 2, 3
    (0x0000, 0x0C, KeplerKeys::Num(1)),
    (0x0000, 0x18, KeplerKeys::Num(2)),
    (0x0000, 0x5E, KeplerKeys::Num(3)),
    // Row 6: 4, 5, 6
    (0x0000, 0x08, KeplerKeys::Num(4)),
    (0x0000, 0x1C, KeplerKeys::Num(5)),
    (0x0000, 0x5A, KeplerKeys::Num(6)),
    // Row 7: 7, 8, 9
    (0x0000, 0x42, KeplerKeys::Num(7)),
    (0x0000, 0x52, KeplerKeys::Num(8)),
    (0x0000, 0x4A, KeplerKeys::Num(9)),
];
