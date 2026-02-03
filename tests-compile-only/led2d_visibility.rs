#![allow(missing_docs)]
//! Compile-only verification that led2d! public/private visibility works.
//!
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use defmt_rtt as _;
use device_envoy::led_strip::{Current, Gamma};
use device_envoy::led2d;
use device_envoy::led2d::Led2dFont;
use device_envoy::led2d::layout::LedLayout;
use embassy_executor::Spawner;
use panic_probe as _;

// Private visibility (no modifier)
const LED_LAYOUT: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

led2d! {
    TestLed {
        pin: PIN_3,
        led_layout: LED_LAYOUT,
        font: Led2dFont::Font3x4Trim,
        max_current: Current::Milliamps(100),
        gamma: Gamma::Linear,
    }
}

/// Compile-time verification that the braced syntax works and generates accessible types.
fn test_led2d_braced_syntax_compilation() {
    // Test that the macro generates the expected types with the new braced syntax
    let _device_size = core::mem::size_of::<TestLed>();

    // This compilation success demonstrates:
    // 1. Braced parameter syntax works (pin: PIN_3, etc.)
    // 2. All required parameters are parsed correctly
    // 3. Generated types are accessible with expected names
    // 4. Generated code compiles without errors
}

// Public visibility in a separate module to avoid PIO conflicts.
mod public_test {
    use super::*;

    const LED_LAYOUT_PUB: LedLayout<12, 4, 3> = LedLayout::serpentine_column_major();

    led2d! {
        pub PublicLed {
            pin: PIN_4,
            pio: PIO1,
            led_layout: LED_LAYOUT_PUB,
            font: Led2dFont::Font3x4Trim,
            max_current: Current::Milliamps(100),
            gamma: Gamma::Linear,
        }
    }

    pub fn use_public_type() {
        let _device_size = core::mem::size_of::<PublicLed>();
    }
}

// Private visibility in a module to ensure it's inaccessible outside.
mod private_test {
    use super::*;

    const LED_LAYOUT_PRIVATE: LedLayout<8, 4, 2> = LedLayout::serpentine_column_major();

    led2d! {
        pub(self) PrivateLed {
            pin: PIN_5,
            pio: PIO0,
            led_layout: LED_LAYOUT_PRIVATE,
            font: Led2dFont::Font3x4Trim,
            max_current: Current::Milliamps(100),
            gamma: Gamma::Linear,
        }
    }

    pub fn use_private_type() {
        let _device_size = core::mem::size_of::<PrivateLed>();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // This main function exists only to satisfy the compiler.
    // The actual verification happens at compile time via the functions above.
    test_led2d_braced_syntax_compilation();
    public_test::use_public_type();
    private_test::use_private_type();

    // Private type should NOT be accessible here (would cause compile error if uncommented):
    // let _private_size = core::mem::size_of::<private_test::PrivateLed>();
}

#[cfg(not(any(target_arch = "arm", target_arch = "riscv32", target_arch = "riscv64")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
