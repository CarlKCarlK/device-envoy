#![allow(missing_docs)]
//! Compile-only verification that IR single-item macros accept optional visibility.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use device_envoy_rp::{Result, ir, ir_kepler, ir_mapping};
use embassy_executor::Spawner;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppButton {
    Power,
}

const APP_BUTTON_MAP: [(u16, u8, AppButton); 1] = [(0x0000, 0x45, AppButton::Power)];

ir! {
    pub IrPublic { pio: PIO0, pin: PIN_15 }
}

ir! {
    IrPrivate { pio: PIO1, pin: PIN_16 }
}

ir_kepler! {
    pub IrKeplerPublic { pio: PIO0, pin: PIN_17 }
}

ir_kepler! {
    IrKeplerPrivate { pio: PIO1, pin: PIN_18 }
}

ir_mapping! {
    pub IrMappingPublic {
        pio: PIO0,
        pin: PIN_19,
        button: AppButton,
        capacity: 1,
    }
}

ir_mapping! {
    IrMappingPrivate {
        pio: PIO1,
        pin: PIN_20,
        button: AppButton,
        capacity: 1,
    }
}

async fn test_ir_visibility(_p: embassy_rp::Peripherals, _spawner: Spawner) -> Result<()> {
    let _ = APP_BUTTON_MAP;
    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
