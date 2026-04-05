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

#[cfg(esp_gdma_family)] // C6, S3, etc
ir_mapping! {
    IrMapping7 {
        pin: GPIO7,
        button: AppButton,
        capacity: 3,
    }
}
#[cfg(esp_pdma_family)] // original ESP32 & s2
ir_mapping! {
    IrMapping7 {
        pin: GPIO4,
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
    let ir_mapping7 = IrMapping7::new(p.GPIO7, channel_creator, &APP_BUTTON_MAP, spawner)?;
    #[cfg(esp_pdma_family)]
    let ir_mapping7 = IrMapping7::new(p.GPIO4, channel_creator, &APP_BUTTON_MAP, spawner)?;

    handle_mapped_button_presses(ir_mapping7).await
}
