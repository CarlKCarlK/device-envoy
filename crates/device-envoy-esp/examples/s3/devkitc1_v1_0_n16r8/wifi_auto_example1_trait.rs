//! Wiring:
//! - Follow the board-specific pin mapping shown in this file.
//!
#![allow(missing_docs)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use esp_backtrace as _;

use device_envoy_esp::{
    Error, Result,
    button::{Button, ButtonEsp, PressedTo},
    flash_block::FlashBlockEsp,
    init_and_start,
    wifi_auto::{WifiAuto, WifiAutoEsp, WifiAutoEvent, WifiStack},
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
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p);

    let [wifi_auto_flash_block] = FlashBlockEsp::new_array::<1>(p.FLASH)?;
    #[cfg(esp_gdma_family)]
    let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
    #[cfg(esp_pdma_family)]
    let mut button6 = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
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
