#![allow(missing_docs)]
//! Compile-only verification that two Led4Rp displays can be created.

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use core::panic::PanicInfo;
use device_envoy_rp::{
    Result,
    led4::{BlinkState, Led4 as _, Led4Rp, Led4RpStatic, OutputArray},
};
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};

async fn test_two_led4(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    let cell_pins0 = OutputArray::new([
        Output::new(p.PIN_0, Level::High),
        Output::new(p.PIN_1, Level::High),
        Output::new(p.PIN_2, Level::High),
        Output::new(p.PIN_3, Level::High),
    ]);
    let segment_pins0 = OutputArray::new([
        Output::new(p.PIN_4, Level::Low),
        Output::new(p.PIN_5, Level::Low),
        Output::new(p.PIN_6, Level::Low),
        Output::new(p.PIN_7, Level::Low),
        Output::new(p.PIN_8, Level::Low),
        Output::new(p.PIN_9, Level::Low),
        Output::new(p.PIN_10, Level::Low),
        Output::new(p.PIN_11, Level::Low),
    ]);

    let cell_pins1 = OutputArray::new([
        Output::new(p.PIN_12, Level::High),
        Output::new(p.PIN_13, Level::High),
        Output::new(p.PIN_14, Level::High),
        Output::new(p.PIN_15, Level::High),
    ]);
    let segment_pins1 = OutputArray::new([
        Output::new(p.PIN_16, Level::Low),
        Output::new(p.PIN_17, Level::Low),
        Output::new(p.PIN_18, Level::Low),
        Output::new(p.PIN_19, Level::Low),
        Output::new(p.PIN_20, Level::Low),
        Output::new(p.PIN_21, Level::Low),
        Output::new(p.PIN_22, Level::Low),
        Output::new(p.PIN_23, Level::Low),
    ]);

    static LED4_0_STATIC: Led4RpStatic = Led4Rp::new_static();
    static LED4_1_STATIC: Led4RpStatic = Led4Rp::new_static();

    let led4_0 = Led4Rp::new(&LED4_0_STATIC, cell_pins0, segment_pins0, spawner)?;
    let led4_1 = Led4Rp::new(&LED4_1_STATIC, cell_pins1, segment_pins1, spawner)?;

    led4_0.write_text(['1', '2', '3', '4'], BlinkState::Solid);
    led4_1.write_text(['a', 'b', 'c', 'd'], BlinkState::Solid);

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
