#![allow(missing_docs)]
//! Compile-only verification that five ButtonRp instances can be created on distinct GPIOs.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::button::{Button as _, ButtonRp, PressedTo};

async fn test_five_buttons(p: embassy_rp::Peripherals) {
    let button0 = ButtonRp::new(p.PIN_0, PressedTo::Ground);
    let button1 = ButtonRp::new(p.PIN_1, PressedTo::Ground);
    let button2 = ButtonRp::new(p.PIN_2, PressedTo::Ground);
    let button3 = ButtonRp::new(p.PIN_3, PressedTo::Ground);
    let button4 = ButtonRp::new(p.PIN_4, PressedTo::Ground);

    let _ = button0.is_pressed();
    let _ = button1.is_pressed();
    let _ = button2.is_pressed();
    let _ = button3.is_pressed();
    let _ = button4.is_pressed();
}

#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
