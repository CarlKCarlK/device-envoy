#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;
use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{Result, ir, ir::Ir as _, ir::IrEvent};
use embassy_executor::Spawner;
use panic_probe as _;

ir! {
    Ir15 { pio: PIO0, pin: PIN_15 }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    info!("IR NEC decoder example starting...");

    let ir15 = Ir15::new(p.PIO0, p.PIN_15, spawner)?;

    info!("IR receiver initialized on GP15");

    // Main loop: process IR events
    loop {
        let event = ir15.wait_for_press().await;
        match event {
            IrEvent::Press { addr, cmd } => {
                info!("IR Button Press - addr=0x{:04X} cmd=0x{:02X}", addr, cmd);
            }
        }
    }
}
