//! Canonical capability definitions shared across platform crates.
//!
//! Platform crates should re-export these items so users can import from
//! `device_envoy_rp` / `device_envoy_esp` instead of from `device_envoy_core`.

/// Compile-time capability key used across platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Wifi,
    Rmt,
    I2s,
    AudioGpio,
    ButtonGpio,
    HighGpioPins,
    ExtendedGpio,
}

/// Bitset of [`Capability`] values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    bits: u32,
}

impl Capability {
    const fn bit(self) -> u32 {
        match self {
            Capability::Wifi => 1 << 0,
            Capability::Rmt => 1 << 1,
            Capability::I2s => 1 << 2,
            Capability::AudioGpio => 1 << 3,
            Capability::ButtonGpio => 1 << 4,
            Capability::HighGpioPins => 1 << 5,
            Capability::ExtendedGpio => 1 << 6,
        }
    }
}

impl CapabilitySet {
    pub const EMPTY: Self = Self { bits: 0 };

    #[must_use]
    pub const fn with(mut self, capability: Capability) -> Self {
        self.bits |= capability.bit();
        self
    }

    #[must_use]
    pub const fn contains(self, capability: Capability) -> bool {
        (self.bits & capability.bit()) != 0
    }
}

/// Platform marker trait for compile-time capability discovery.
pub trait PlatformCapabilities {
    const CAPABILITIES: CapabilitySet;
}
