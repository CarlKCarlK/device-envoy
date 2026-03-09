#![allow(missing_docs)]
//! Compile-only verification that four raw IR receivers on one PIO compile.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::irs;
use embassy_executor::Spawner;

irs! {
    pio: PIO0,
    Irs0 {
        Ir15: { pin: PIN_15 },
        Ir16: { pin: PIN_16 },
        Ir17: { pin: PIN_17 },
        Ir18: { pin: PIN_18 }
    }
}

async fn test_four_irs(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (_ir15, _ir16, _ir17, _ir18) =
        Irs0::new(p.PIO0, p.PIN_15, p.PIN_16, p.PIN_17, p.PIN_18, spawner)?;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
