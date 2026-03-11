#![allow(missing_docs)]
//! Compile-only verification that five macro-generated Led devices can be created.

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

led!(LedOne { pin: PIN_1 });
led!(LedTwo {
    pin: PIN_2,
    max_steps: 40
});
led!(LedThree { pin: PIN_3 });
led!(LedFour { pin: PIN_4 });
led!(LedFive { pin: PIN_5 });

async fn test_five_leds(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let led_one = LedOne::new(p.PIN_1, OnLevel::High, spawner)?;
    let led_two = LedTwo::new(p.PIN_2, OnLevel::High, spawner)?;
    let led_three = LedThree::new(p.PIN_3, OnLevel::High, spawner)?;
    let led_four = LedFour::new(p.PIN_4, OnLevel::High, spawner)?;
    let led_five = LedFive::new(p.PIN_5, OnLevel::High, spawner)?;

    led_one.set_level(LedLevel::On);
    led_two.set_level(LedLevel::Off);
    led_three.set_level(LedLevel::On);
    led_four.set_level(LedLevel::Off);
    led_five.set_level(LedLevel::On);

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
