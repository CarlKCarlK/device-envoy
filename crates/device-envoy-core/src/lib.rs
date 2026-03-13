#![cfg_attr(target_os = "none", no_std)]

//! Shared building blocks for the device-envoy workspace.
pub mod audio_player;
pub mod button;
pub mod clock;
#[cfg(feature = "wifi")]
pub mod clock_sync;
pub mod flash_block;
pub mod ir;
pub mod lcd_text;
pub mod led;
pub mod led2d;
pub mod led4;
pub mod led_strip;
pub mod rfid;
pub mod servo;
#[doc(hidden)]
pub mod servo_player;
#[cfg(feature = "wifi")]
pub mod time_sync;
#[cfg(feature = "host")]
pub mod to_png;
pub mod wifi_auto;

/// Used internally by other macros.
#[doc(hidden)]
pub use paste::paste as __paste;
