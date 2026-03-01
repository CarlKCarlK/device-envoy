#![doc = include_str!("../README.md")]
//!
//! # Glossary
//!
//! Resources available on the Pico 1 and Pico 2:
//!
//! - **PIO ([Programmable I/O](https://medium.com/data-science/nine-pico-pio-wats-with-rust-part-1-9d062067dc25)):** Pico 1 has 2. Pico 2 has 3.
//! - **DMA ([Direct Memory Access](https://en.wikipedia.org/wiki/Direct_memory_access)):** Both Pico 1 and 2 have 12 channels.
//! - **PWM ([Pulse Width Modulation](https://en.wikipedia.org/wiki/Pulse-width_modulation)) Slices:** Both  Pico 1 and 2 have 8 slices (& 16 channels). These "slices"
//!   are unrelated Rust slices.
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(async_fn_in_trait, reason = "single-threaded embedded")]

// Compile-time checks: exactly one board must be selected (unless testing with host feature)
#[cfg(all(target_os = "none", not(any(feature = "pico1", feature = "pico2"))))]
compile_error!("Must enable exactly one board feature: 'pico1' or 'pico2'");

#[cfg(all(target_os = "none", feature = "pico1", feature = "pico2"))]
compile_error!("Cannot enable both 'pico1' and 'pico2' features simultaneously");

// Compile-time checks: exactly one architecture must be selected (unless testing with host feature)
#[cfg(all(target_os = "none", not(any(feature = "arm", feature = "riscv"))))]
compile_error!("Must enable exactly one architecture feature: 'arm' or 'riscv'");

#[cfg(all(target_os = "none", feature = "arm", feature = "riscv"))]
compile_error!("Cannot enable both 'arm' and 'riscv' features simultaneously");

// Compile-time check: pico1 only supports ARM
#[cfg(all(target_os = "none", feature = "pico1", feature = "riscv"))]
compile_error!("Pico 1 (RP2040) only supports ARM architecture, not RISC-V");

#[cfg(target_os = "none")]
pub(crate) mod bit_matrix_led4;
// PIO interrupt bindings - shared by led_strip::strip and led_strip
#[cfg(target_os = "none")]
#[doc(hidden)]
pub mod pio_irqs;
#[cfg(feature = "host")]
/// Utilities for converting frames to PNG images (host testing only).
pub mod to_png;
// Embedded-only in normal builds, but compiled for host unit tests.
#[cfg(any(target_os = "none", all(test, feature = "host")))]
pub mod audio_player;
#[cfg(target_os = "none")]
pub mod button;
#[cfg(target_os = "none")]
pub mod char_lcd;
#[cfg(all(feature = "wifi", target_os = "none"))]
pub(crate) mod clock;
#[cfg(all(feature = "wifi", target_os = "none"))]
pub mod clock_sync;
mod error;
#[cfg(target_os = "none")]
pub mod flash_array;
#[cfg(target_os = "none")]
pub mod ir;
#[cfg(target_os = "none")]
pub mod led;
pub mod led2d;
#[cfg(target_os = "none")]
pub mod led4;
pub mod led_strip;
#[cfg(target_os = "none")]
pub mod rfid;
#[cfg(target_os = "none")]
pub mod servo;
#[cfg(target_os = "none")]
pub mod servo_player;
#[cfg(target_os = "none")]
pub(crate) mod time_sync;
#[cfg(all(feature = "wifi", target_os = "none"))]
pub mod wifi_auto;

// Re-export error types and result (used throughout)
pub use crate::error::{Error, Result};
