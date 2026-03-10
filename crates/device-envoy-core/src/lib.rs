#![cfg_attr(target_os = "none", no_std)]

//! Shared building blocks for the device-envoy workspace.
// TODO0 Audit remaining StaticCell usage across device abstractions and remove
// TODO0 it where const/static initialization can replace runtime init safely.
// TODO0 Audit all device macros to ensure optional keyword fields are accepted
// TODO0 in any order, with consistent duplicate/unknown-field diagnostics.

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
