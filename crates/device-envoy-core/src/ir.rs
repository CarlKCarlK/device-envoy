//! A device abstraction for infrared receivers using the NEC protocol.
//!
//! This module provides platform-independent types and helpers for NEC IR decoding.
//! See the platform-specific crate (for example `device_envoy_rp::ir` or
//! `device_envoy_esp::ir`) for the primary documentation and examples.

pub mod kepler;
pub mod mapping;

pub use kepler::{IrKeplerStatic, KEPLER_MAPPING, KeplerButton};
pub use mapping::IrMappingStatic;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel as EmbassyChannel;

/// Events received from the infrared receiver.
///
/// See the platform-specific crate for usage examples.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IrEvent {
    /// Button press with 16-bit address and 8-bit command.
    /// Supports both standard NEC (8-bit address) and extended NEC (16-bit address).
    Press {
        /// 16-bit device address (or 8-bit address in low byte for standard NEC).
        addr: u16,
        /// 8-bit command code.
        cmd: u8,
    },
}

/// Static resources for the [`Ir`] device abstraction.
///
/// See the platform-specific crate for usage examples.
pub struct IrStatic(EmbassyChannel<CriticalSectionRawMutex, IrEvent, 8>);

impl IrStatic {
    /// Creates static resources for the infrared receiver device.
    #[must_use]
    pub const fn new() -> Self {
        Self(EmbassyChannel::new())
    }

    /// Send an IR event to waiting receivers.
    ///
    /// Called by the platform's hardware receiver task.
    pub async fn send(&self, event: IrEvent) {
        self.0.send(event).await;
    }

    /// Wait for the next IR event.
    pub async fn receive(&self) -> IrEvent {
        self.0.receive().await
    }
}

/// Decode and validate a 32-bit NEC frame.
///
/// NEC protocol structure (32 bits, LSB first):
/// - Byte 0: Address (8 bits)
/// - Byte 1: Address inverse (~Address)
/// - Byte 2: Command (8 bits)
/// - Byte 3: Command inverse (~Command)
///
/// Extended NEC uses 16-bit address (bytes 0-1) without inversion check.
///
/// Returns `Some((address, command))` if valid, `None` if checksum fails.
pub fn decode_nec_frame(frame: u32) -> Option<(u16, u8)> {
    let byte0 = (frame & 0xFF) as u8;
    let byte1 = ((frame >> 8) & 0xFF) as u8;
    let byte2 = ((frame >> 16) & 0xFF) as u8;
    let byte3 = ((frame >> 24) & 0xFF) as u8;

    // Validate command bytes (required in both standard and extended NEC)
    if (byte2 ^ byte3) != 0xFF {
        return None;
    }

    // Standard NEC: 8-bit address with inverse validation
    if (byte0 ^ byte1) == 0xFF {
        return Some((u16::from(byte0), byte2));
    }

    // Extended NEC: 16-bit address (no inversion check on address)
    let addr16 = ((u16::from(byte1)) << 8) | u16::from(byte0);
    Some((addr16, byte2))
}
