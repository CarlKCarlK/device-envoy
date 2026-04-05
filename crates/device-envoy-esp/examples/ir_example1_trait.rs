#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start, ir,
    ir::{Ir, IrEvent},
    Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
ir! {
    Ir7 { pin: GPIO7 }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
ir! {
    Ir7 { pin: GPIO4 }
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
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Async);

    // ESP32-S3 requires RX channel 4+.
    #[cfg(feature = "esp32s3")]
    let channel_creator = rmt80.channel4;
    #[cfg(not(feature = "esp32s3"))]
    let channel_creator = rmt80.channel2;
    #[cfg(esp_gdma_family)]
    let ir7 = Ir7::new(p.GPIO7, channel_creator, spawner)?;
    #[cfg(esp_pdma_family)]
    let ir7 = Ir7::new(p.GPIO4, channel_creator, spawner)?;

    handle_ir_presses(ir7).await
}
