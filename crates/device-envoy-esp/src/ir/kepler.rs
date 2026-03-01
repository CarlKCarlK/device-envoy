//! A device abstraction for the SunFounder Kepler Kit IR remote control.
//!
//! See [`IrKepler`] for usage examples.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use embassy_executor::Spawner;

use crate::ir::mapping::{IrMapping, IrMappingStatic};
#[cfg(target_os = "none")]
use crate::Result;

/// Button types for the SunFounder Kepler Kit remote control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Static resources for Kepler IR remote events.
pub struct IrKeplerStatic(IrMappingStatic);

impl IrKeplerStatic {
    /// Create static resources for the Kepler remote.
    #[must_use]
    pub const fn new() -> Self {
        Self(IrMappingStatic::new())
    }

    pub(crate) const fn inner(&self) -> &IrMappingStatic {
        &self.0
    }
}

type IrKeplerMapping<'a> = IrMapping<'a, KeplerButton, 21>;

const KEPLER_MAPPING: [(u16, u8, KeplerButton); 21] = [
    (0x0000, 0x45, KeplerButton::Power),
    (0x0000, 0x46, KeplerButton::Mode),
    (0x0000, 0x47, KeplerButton::Mute),
    (0x0000, 0x44, KeplerButton::PlayPause),
    (0x0000, 0x40, KeplerButton::Prev),
    (0x0000, 0x43, KeplerButton::Next),
    (0x0000, 0x07, KeplerButton::Eq),
    (0x0000, 0x15, KeplerButton::Minus),
    (0x0000, 0x09, KeplerButton::Plus),
    (0x0000, 0x16, KeplerButton::Num(0)),
    (0x0000, 0x19, KeplerButton::Repeat),
    (0x0000, 0x0D, KeplerButton::USd),
    (0x0000, 0x0C, KeplerButton::Num(1)),
    (0x0000, 0x18, KeplerButton::Num(2)),
    (0x0000, 0x5E, KeplerButton::Num(3)),
    (0x0000, 0x08, KeplerButton::Num(4)),
    (0x0000, 0x1C, KeplerButton::Num(5)),
    (0x0000, 0x5A, KeplerButton::Num(6)),
    (0x0000, 0x42, KeplerButton::Num(7)),
    (0x0000, 0x52, KeplerButton::Num(8)),
    (0x0000, 0x4A, KeplerButton::Num(9)),
];

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
