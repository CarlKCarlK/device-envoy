#![allow(missing_docs)]
//! Example showing how to use the SunFounder Kepler Kit IR remote.
#![no_std]
#![no_main]
use core::{convert::Infallible, future::pending};

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{Result, ir::IrKepler as _, ir_keplers};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use panic_probe as _;

ir_keplers! {
    pio: PIO0,
    IrKeplers0 {
        IrKepler15: { pin: PIN_15 },
        IrKepler01: { pin: PIN_16 }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("Starting Kepler IR Remote Example");

    let (ir_kepler15, ir_kepler01) = IrKeplers0::new(p.PIO0, p.PIN_15, p.PIN_16, spawner)?;

    info!("Kepler remotes initialized on GPIO 15 and 16");
    info!("Press buttons on the remote control...");

    loop {
        match select(ir_kepler15.wait_for_press(), ir_kepler01.wait_for_press()).await {
            Either::First(button0) => info!("Kepler15 button pressed: {:?}", button0),
            Either::Second(button1) => info!("Kepler01 button pressed: {:?}", button1),
        }
    }
}
