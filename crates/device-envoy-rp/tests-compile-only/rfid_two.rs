#![allow(missing_docs)]
//! Compile-only verification that two RFID readers can be constructed (SPI0 + SPI1).

#![cfg(not(feature = "host"))]
#![no_std]
#![no_main]
#![allow(dead_code, reason = "Compile-time verification only")]

use core::panic::PanicInfo;

use device_envoy_rp::{
    Result,
    rfid::{RfidRp, RfidStatic},
};
use embassy_executor::Spawner;

async fn test_two_rfid(p: embassy_rp::Peripherals, spawner: Spawner) -> Result<()> {
    static RFID0_STATIC: RfidStatic = RfidRp::new_static();
    let _rfid0 = RfidRp::new_spi0(
        &RFID0_STATIC,
        p.SPI0,
        p.PIN_2,
        p.PIN_3,
        p.PIN_4,
        p.DMA_CH0,
        p.DMA_CH1,
        p.PIN_1,
        p.PIN_5,
        spawner,
    )
    .await?;

    static RFID1_STATIC: RfidStatic = RfidRp::new_static();
    let _rfid1 = RfidRp::new_spi1(
        &RFID1_STATIC,
        p.SPI1,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.DMA_CH2,
        p.DMA_CH3,
        p.PIN_9,
        p.PIN_13,
        spawner,
    )
    .await?;

    Ok(())
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
