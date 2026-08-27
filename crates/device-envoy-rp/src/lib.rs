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
//! - **I2C ([Inter-Integrated Circuit](https://en.wikipedia.org/wiki/I%C2%B2C)):** Serial control bus used by abstractions like `lcd_text`. Pico 1 and Pico 2 each provide 2 controllers (`I2C0`, `I2C1`).
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(async_fn_in_trait, reason = "single-threaded embedded")]

// Compile-time checks: exactly one board must be selected (unless testing with host feature)
#[cfg(all(target_os = "none", not(any(feature = "pico1", feature = "pico2"))))]
compile_error!("Must enable exactly one board feature: 'pico1' or 'pico2'");

#[cfg(all(target_os = "none", feature = "pico1", feature = "pico2"))]
compile_error!("Cannot enable both 'pico1' and 'pico2' features simultaneously");

// Compile-time checks: RP support is ARM-only.
#[cfg(all(target_os = "none", not(feature = "arm")))]
compile_error!("Must enable architecture feature: 'arm'");

#[cfg(all(
    target_os = "none",
    any(target_arch = "riscv32", target_arch = "riscv64")
))]
compile_error!("RISC-V targets are not supported for device-envoy-rp");

// PIO interrupt bindings - shared by led_strip::strip and led_strip
#[cfg(target_os = "none")]
#[doc(hidden)]
pub mod pio_irqs;
// Embedded-only in normal builds, but compiled for host unit tests.
#[cfg(any(target_os = "none", all(test, feature = "host")))]
pub mod audio_player;
#[cfg(target_os = "none")]
pub mod button;
#[cfg(all(feature = "wifi", target_os = "none"))]
pub mod clock_sync;
#[cfg(target_os = "none")]
pub mod cyd;
// The buffer implementation is hardware-independent, so exercise the same
// private storage code in the host test harness even though the full CYD
// peripheral module is embedded-only.
#[cfg(all(test, feature = "host"))]
#[path = "cyd/buffer.rs"]
mod cyd_buffer_host_tests;
mod error;
#[cfg(target_os = "none")]
pub mod flash_block;
#[cfg(target_os = "none")]
pub mod ir;
#[cfg(target_os = "none")]
pub mod lcd_text;
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
mod servo_player;
#[cfg(all(feature = "wifi", target_os = "none"))]
pub mod wifi_auto;

#[cfg(doc)]
pub mod docs {
    //! Documentation-only pages for this crate.
    #[doc = include_str!("docs/development_guide.md")]
    pub mod development_guide {}
}

// Re-export error types and result (used throughout)
pub use crate::error::{Error, Result};
pub use device_envoy_core::tone;
/// Used internally by other macros.
#[doc(hidden)]
pub use paste::paste as __paste;

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __validate_keyword_fields_expr {
    (
        macro_name: $macro_name:literal,
        allowed_macro: $allowed_macro:path,
        fields: [ $( $field:ident : $value:expr ),* $(,)? ]
    ) => {
        const _: () = {
            $( $allowed_macro!($field, $macro_name); )*
            #[allow(non_snake_case)]
            mod __device_envoy_keyword_fields_uniqueness {
                $( pub(super) mod $field {} )*
            }
        };
    };

    (
        macro_name: $macro_name:literal,
        allowed_macro: $allowed_macro:path,
        fields: [ $($fields:tt)* ]
    ) => {
        compile_error!(concat!($macro_name, " fields must use `name: value` syntax"));
    };
}
