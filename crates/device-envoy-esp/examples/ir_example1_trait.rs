#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    init_and_start::rmt_mode,
    ir,
    ir::{Ir, IrEvent},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

ir! {
    Ir7: { pin: GPIO7 }
}

async fn handle_ir_presses(ir: &impl Ir) -> ! {
    loop {
        let ir_event = ir.wait_for_press().await;
        match ir_event {
            IrEvent::Press { addr, cmd } => {
                // Handle decoded NEC press event.
                let _ = (addr, cmd);
            }
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    // On ESP32-S3, RMT channels 0-3 are TX-only; RX requires channel 4+.
    // On ESP32-C6, channels 0-3 all support RX.
    #[cfg(target_arch = "xtensa")]
    let channel_creator = rmt80.channel4;
    #[cfg(not(target_arch = "xtensa"))]
    let channel_creator = rmt80.channel2;
    let ir7 = Ir7::new(p.GPIO7, channel_creator, spawner)?;

    handle_ir_presses(ir7).await
}
