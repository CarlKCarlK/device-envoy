#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{
    init_and_start,
    ir::{IrKepler, KeplerKeys},
    ir_kepler, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(esp_gdma_family)] // C6, S3, etc
ir_kepler! {
    IrKepler7 { pin: GPIO7 }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
ir_kepler! {
    IrKepler7 { pin: GPIO4 }
}

async fn handle_kepler_button_presses(ir_kepler: &impl IrKepler) -> ! {
    loop {
        let kepler_button = ir_kepler.wait_for_press().await;
        match kepler_button {
            KeplerKeys::Power => {
                // Handle power.
            }
            KeplerKeys::PlayPause => {
                // Handle play/pause.
            }
            _ => {
                // Handle all other Kepler buttons.
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
    #[cfg(esp_gdma_family)]
    let ir_kepler7 = IrKepler7::new(p.GPIO7, channel_creator, spawner)?;
    #[cfg(esp_pdma_family)]
    let ir_kepler7 = IrKepler7::new(p.GPIO4, channel_creator, spawner)?;

    handle_kepler_button_presses(ir_kepler7).await
}
