//! Compile-only verification that fields of kind `expr` accept compound expressions
//! where the generated code uses them as const generic arguments.
//!
//! Run via: `cargo check-all` (xtask compiles this for thumbv6m-none-eabi)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::audio_player::{AudioPlayer, VOICE_22050_HZ, audio_player};
use device_envoy_rp::ir::{IrMapping, ir_mapping};
use device_envoy_rp::led_strip::{LedStrip, led_strip};
use device_envoy_rp::servo::{ServoPlayer, servo_player};
use embassy_executor::Spawner;

/// The keys `ExprIr` maps codes to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Key {
    /// Power key.
    Power,
}

led_strip! {
    /// A strip whose length is an expression.
    ExprStrip { pin: PIN_2, len: 4 * 2, max_frames: 1 + 1 }
}

servo_player! {
    /// A servo player whose step count is an expression.
    ExprServo { pin: PIN_10, max_steps: 10 * 3 }
}

ir_mapping! {
    /// An IR mapping whose capacity is an expression.
    ExprIr { pio: PIO1, pin: PIN_15, button: Key, capacity: 2 * 2 }
}

audio_player! {
    /// An audio player whose clip count is an expression.
    ExprAudio {
        data_pin: PIN_8,
        bit_clock_pin: PIN_9,
        word_select_pin: PIN_10,
        sample_rate_hz: VOICE_22050_HZ,
        max_clips: 2 * 4,
    }
}

// The expressions reach the trait impls with their values.
/// Requires an 8-LED strip.
const fn strip_is<T: LedStrip<8>>() {}
/// Requires a 30-step servo player.
const fn servo_is<T: ServoPlayer<30>>() {}
/// Requires an IR mapping to `Key`.
const fn ir_is<T: IrMapping<Key>>() {}
/// Requires a 22.05 kHz audio player.
const fn audio_is<T: AudioPlayer<VOICE_22050_HZ>>() {}
const _: () = {
    strip_is::<ExprStrip>();
    servo_is::<ExprServo>();
    ir_is::<ExprIr>();
    audio_is::<ExprAudio>();
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _key = Key::Power;
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
