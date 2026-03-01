//! A device abstraction for NeoPixel-style (WS2812) LED strips.
//!
//! Platform-independent types ([`Frame1d`], [`Gamma`], [`LedStrip`], color
//! types, etc.) come from [`device_envoy_core::led_strip`] and are re-exported
//! here for transparent access.
//!
//! The [`esp32`] sub-module provides the RMT driver, device loop, and the
//! `led_strip!` macro that wires them together. The [`esp32_spi`] sub-module
//! provides the SPI variant.

pub use device_envoy_core::led_strip::*;

/// Output engine for WS2812 transmission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine {
    /// ESP32 RMT pulse engine.
    Rmt,
    /// ESP32 SPI byte-stream engine.
    Spi,
}

impl Default for Engine {
    fn default() -> Self {
        Self::Rmt
    }
}

// ============================================================================
// ESP32-specific sub-module
// ============================================================================

#[cfg(target_os = "none")]
pub mod esp32;

#[cfg(target_os = "none")]
pub mod esp32_spi;
