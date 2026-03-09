#![allow(missing_docs)]
//! Compile-only verification that four Kepler IR receivers on one PIO compile.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::Result;
use device_envoy_rp::ir_keplers;
use embassy_executor::Spawner;

ir_keplers! {
    pio: PIO0,
    IrKeplers0 {
        IrKepler15: { pin: PIN_15 },
        IrKepler16: { pin: PIN_16 },
        IrKepler17: { pin: PIN_17 },
        IrKepler18: { pin: PIN_18 }
    }
}

async fn test_four_ir_keplers(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let (_ir_kepler15, _ir_kepler16, _ir_kepler17, _ir_kepler18) =
        IrKeplers0::new(p.PIO0, p.PIN_15, p.PIN_16, p.PIN_17, p.PIN_18, spawner)?;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
