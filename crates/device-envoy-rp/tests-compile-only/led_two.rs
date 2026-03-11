#![allow(missing_docs)]
//! Compile-only verification that two macro-generated Led devices can be created.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use core::panic::PanicInfo;

use device_envoy_rp::{
    Result, led,
    led::{Led as _, LedLevel, OnLevel},
};
use embassy_executor::Spawner;

led!(LedAlpha { pin: PIN_1 });
led!(LedBeta {
    pin: PIN_2,
    max_steps: 2,
});

async fn test_two_led(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let led_alpha = LedAlpha::new(p.PIN_1, OnLevel::High, spawner)?;
    let led_beta = LedBeta::new(p.PIN_2, OnLevel::High, spawner)?;

    led_alpha.set_level(LedLevel::On);
    led_beta.set_level(LedLevel::Off);

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
