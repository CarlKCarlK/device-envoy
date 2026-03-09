#![allow(missing_docs)]
//! Compile-only negative test: reusing one singleton `ir_kepler!` constructor with
//! the same owned PIO resource should fail to compile.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]

use device_envoy_rp::Result;
use device_envoy_rp::ir_kepler;
use embassy_executor::Spawner;

ir_kepler! {
    IrKepler15: { pio: PIO0, pin: PIN_15 }
}

async fn test_reuse_single_ir_kepler(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let _first = IrKepler15::new(p.PIO0, p.PIN_15, spawner)?;
    let _second = IrKepler15::new(p.PIO0, p.PIN_15, spawner)?;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
