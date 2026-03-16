#![no_std]
#![doc = include_str!("../README.md")]

/// Marker type for the top-level `device-envoy` landing crate.
///
/// This crate intentionally exposes minimal API surface and points users to
/// platform crates.
pub struct DeviceEnvoy;
