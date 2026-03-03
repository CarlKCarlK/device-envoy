#![cfg_attr(target_os = "none", no_std)]

//! Shared building blocks for the device-envoy workspace.

pub mod audio_player;
pub mod clock;
#[cfg(feature = "wifi")]
pub mod clock_sync;
pub mod flash_array;
pub mod ir;
pub mod led2d;
pub mod led_strip;
#[cfg(feature = "wifi")]
pub mod time_sync;
#[cfg(feature = "host")]
pub mod to_png;
pub mod wifi_auto;
