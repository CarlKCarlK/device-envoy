//! Compile-fail verification: a servo given a PWM slice that does not drive its pin.
//!
//! `PIN_11` is on `PWM_SLICE5`, so passing `PWM_SLICE4` must fail with `ServoPwmSlice`'s
//! message.
//!
//! Run via: `cargo check-all` (xtask expects this to fail to compile)

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::servo::servo;
use embassy_executor::Spawner;

servo! {
    /// A servo on `PIN_11`.
    Servo11 { pin: PIN_11 }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let _servo = Servo11::new(p.PIN_11, p.PWM_SLICE4);
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
