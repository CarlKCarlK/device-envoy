#![allow(missing_docs)]
#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

//! Two single-strip devices on the same PIO resource in one module must not compile:
//! a PIO resource belongs to exactly one `led_strip!`, `led2d!`, or `led_strips!` group.

use defmt_rtt as _;

use device_envoy_rp::led_strip::led_strip;
use embassy_executor::Spawner;

led_strip! {
    pub FirstStrip { pin: PIN_0, len: 8 }
}

led_strip! {
    pub SecondStrip { pin: PIN_1, len: 8, dma: DMA_CH1 }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // This main function exists only to satisfy the compiler.
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
