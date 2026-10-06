#![allow(missing_docs)]
#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

//! An LED strip and an IR receiver, in different modules, both on the default PIO
//! (`PIO0`) must not compile: the second constructor gets a PIO resource that the first
//! already owns (`use of moved value: p.PIO0`).

use defmt_rtt as _;

use device_envoy_rp::Result;
use embassy_executor::Spawner;

mod leds {
    device_envoy_rp::led_strip::led_strip! {
        pub Strip { pin: PIN_0, len: 8 }
    }
}

mod remotes {
    device_envoy_rp::ir::ir! {
        pub Remote { pin: PIN_15 }
    }
}

async fn build(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let _strip = leds::Strip::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;
    let _remote = remotes::Remote::new(p.PIO0, p.PIN_15, spawner)?;
    Ok(())
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
