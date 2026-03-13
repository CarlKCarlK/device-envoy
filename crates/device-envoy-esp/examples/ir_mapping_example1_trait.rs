#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{init_and_start, ir::IrMapping, ir_mapping, Result};

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppButton {
    Power,
    Plus,
    Minus,
}

const APP_BUTTON_MAP: [(u16, u8, AppButton); 3] = [
    (0x0000, 0x45, AppButton::Power),
    (0x0000, 0x09, AppButton::Plus),
    (0x0000, 0x15, AppButton::Minus),
];

ir_mapping! {
    IrMapping7 {
        pin: GPIO7,
        button: AppButton,
        capacity: 3,
    }
}

async fn handle_mapped_button_presses(ir_mapping: &impl IrMapping<AppButton>) -> ! {
    loop {
        let app_button = ir_mapping.wait_for_press().await;
        match app_button {
            AppButton::Power => {
                // Handle power.
            }
            AppButton::Plus => {
                // Handle plus.
            }
            AppButton::Minus => {
                // Handle minus.
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
    let ir_mapping7 = IrMapping7::new(p.GPIO7, channel_creator, &APP_BUTTON_MAP, spawner)?;

    handle_mapped_button_presses(ir_mapping7).await
}
