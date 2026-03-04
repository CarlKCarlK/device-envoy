//! Platform-independent types for the SunFounder Kepler Kit IR remote control.
//!
//! See the platform-specific crate for primary documentation and examples.

use crate::ir::mapping::IrMappingStatic;

/// Button types for the SunFounder Kepler Kit remote control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KeplerButton {
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
#[allow(async_fn_in_trait)]
pub trait IrKeplerDevice {
    /// Wait for the next recognized Kepler button press.
    async fn wait_for_press(&self) -> KeplerButton;
}

/// Static resources for Kepler IR remote events.
///
/// See the platform-specific crate for usage examples.
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

/// Button mapping for the SunFounder Kepler Kit remote (ordered to match physical layout).
pub const KEPLER_MAPPING: [(u16, u8, KeplerButton); 21] = [
    // Row 1: Power, Mode, Mute
    (0x0000, 0x45, KeplerButton::Power),
    (0x0000, 0x46, KeplerButton::Mode),
    (0x0000, 0x47, KeplerButton::Mute),
    // Row 2: PlayPause, Prev, Next
    (0x0000, 0x44, KeplerButton::PlayPause),
    (0x0000, 0x40, KeplerButton::Prev),
    (0x0000, 0x43, KeplerButton::Next),
    // Row 3: EQ, Minus, Plus
    (0x0000, 0x07, KeplerButton::Eq),
    (0x0000, 0x15, KeplerButton::Minus),
    (0x0000, 0x09, KeplerButton::Plus),
    // Row 4: 0, Repeat, U/SD
    (0x0000, 0x16, KeplerButton::Num(0)),
    (0x0000, 0x19, KeplerButton::Repeat),
    (0x0000, 0x0D, KeplerButton::USd),
    // Row 5: 1, 2, 3
    (0x0000, 0x0C, KeplerButton::Num(1)),
    (0x0000, 0x18, KeplerButton::Num(2)),
    (0x0000, 0x5E, KeplerButton::Num(3)),
    // Row 6: 4, 5, 6
    (0x0000, 0x08, KeplerButton::Num(4)),
    (0x0000, 0x1C, KeplerButton::Num(5)),
    (0x0000, 0x5A, KeplerButton::Num(6)),
    // Row 7: 7, 8, 9
    (0x0000, 0x42, KeplerButton::Num(7)),
    (0x0000, 0x52, KeplerButton::Num(8)),
    (0x0000, 0x4A, KeplerButton::Num(9)),
];
