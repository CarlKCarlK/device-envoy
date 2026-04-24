#![doc = include_str!("../README.md")]
#![cfg_attr(target_os = "none", no_std)]

pub mod audio_player;
pub mod button;
pub mod error;
#[cfg(feature = "wifi")]
pub(crate) mod clock;
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
mod to_png;
pub mod wifi_auto;

pub use error::{Error, Result};

/// Used internally by other macros.
#[doc(hidden)]
pub use paste::paste as __paste;
