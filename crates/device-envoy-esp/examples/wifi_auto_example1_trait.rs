#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{
    button::{Button, ButtonEsp, PressedTo},
    flash_block::FlashBlockEsp,
    init_and_start,
    wifi_auto::{WifiAuto, WifiAutoEsp, WifiAutoEvent, WifiStack},
    Error, Result,
};

esp_bootloader_esp_idf::esp_app_desc!();

async fn connect_with_status(
    wifi_auto: impl WifiAuto<Error = Error>,
    button: &mut impl Button,
) -> Result<WifiStack> {
    wifi_auto
        .connect(button, |wifi_auto_event| async move {
            match wifi_auto_event {
                WifiAutoEvent::CaptivePortalReady => {
                    // Captive portal is ready for Wi-Fi credential entry.
                }
                WifiAutoEvent::Connecting { .. } => {
                    // A Wi-Fi connection attempt is in progress.
                }
                WifiAutoEvent::ConnectionFailed => {
                    // All connection attempts failed.
                }
            }
            Ok(())
        })
        .await
}

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(error) => panic!("{error:?}"),
    }
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p);

    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        "DeviceEnvoySetup",
        [],
        spawner,
    )?;

    let _stack = connect_with_status(wifi_auto, &mut button6).await?;

    core::future::pending().await
}
