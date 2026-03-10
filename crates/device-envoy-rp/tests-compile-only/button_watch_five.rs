#![allow(missing_docs)]
//! Compile-only verification that five button_watch devices can be created on distinct GPIOs.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::button::{Button as _, PressedTo};
use device_envoy_rp::button_watch;
use embassy_executor::Spawner;

button_watch! {
    ButtonWatch0 { pin: PIN_0, }
}

button_watch! {
    ButtonWatch1 { pin: PIN_1, }
}

button_watch! {
    ButtonWatch2 { pin: PIN_2, }
}

button_watch! {
    ButtonWatch3 { pin: PIN_3, }
}

button_watch! {
    ButtonWatch4 { pin: PIN_4, }
}

async fn test_five_button_watches(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let button_watch0 = ButtonWatch0::new(p.PIN_0, PressedTo::Ground, spawner)?;
    let button_watch1 = ButtonWatch1::new(p.PIN_1, PressedTo::Ground, spawner)?;
    let button_watch2 = ButtonWatch2::new(p.PIN_2, PressedTo::Ground, spawner)?;
    let button_watch3 = ButtonWatch3::new(p.PIN_3, PressedTo::Ground, spawner)?;
    let button_watch4 = ButtonWatch4::new(p.PIN_4, PressedTo::Ground, spawner)?;

    let _ = button_watch0.is_pressed();
    let _ = button_watch1.is_pressed();
    let _ = button_watch2.is_pressed();
    let _ = button_watch3.is_pressed();
    let _ = button_watch4.is_pressed();
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
