//! Embedded compile-only test target: fields of kind `expr` accept compound expressions
//! where the generated code uses them as const generic arguments.

#![no_std]
#![no_main]
#![deny(warnings, missing_docs)]

use core::convert::Infallible;
use embassy_executor::Spawner;
use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    ir::{IrMapping, ir_mapping},
    led_strip::{LedStrip, led_strip},
    servo::{ServoPlayer, servo_player},
};

esp_bootloader_esp_idf::esp_app_desc!();

/// The keys `ExprIr` maps codes to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Key {
    /// Power key.
    Power,
}

led_strip! {
    /// A strip whose length is an expression.
    ExprStrip { pin: GPIO2, len: 4 * 2, max_frames: 1 + 1 }
}

servo_player! {
    /// A servo player whose step count is an expression.
    ExprServo { pin: GPIO3, timer: Timer0, channel: Channel0, max_steps: 10 * 3 }
}

ir_mapping! {
    /// An IR mapping whose capacity is an expression.
    ExprIr { pin: GPIO4, button: Key, capacity: 2 * 2 }
}

// The expressions reach the trait impls with their values.
/// Requires an 8-LED strip.
const fn strip_is<T: LedStrip<8>>() {}
/// Requires a 30-step servo player.
const fn servo_is<T: ServoPlayer<30>>() {}
/// Requires an IR mapping to `Key`.
const fn ir_is<T: IrMapping<Key>>() {}
const _: () = {
    strip_is::<ExprStrip>();
    servo_is::<ExprServo>();
    ir_is::<ExprIr>();
};

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(_spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);
    let _key = Key::Power;
    core::future::pending().await
}
