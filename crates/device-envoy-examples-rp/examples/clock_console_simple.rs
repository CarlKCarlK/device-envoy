#![allow(missing_docs)]
//! Minimal Wi-Fi clock that logs the local time once per minute.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::convert::Infallible;

use defmt::info;
use defmt_rtt as _;
use device_envoy_rp::{
    Error, Result,
    button::{ButtonRp, PressedTo},
    clock_sync::{ClockSync as _, ClockSyncRp, ClockSyncStaticRp, ONE_MINUTE},
    flash_block::FlashBlockRp,
    wifi_auto::{
        WifiAutoEvent, WifiAutoRp,
        fields::{TimezoneField, TimezoneFieldStatic},
    },
};
use embassy_executor::Spawner;
use panic_probe as _;

#[embassy_executor::main]
pub async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    core::panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    info!("Starting simple console clock");
    let p = embassy_rp::init(Default::default());

    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashBlockRp::new_array::<2>(p.FLASH)?;

    static TIMEZONE_FIELD_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_FIELD_STATIC, timezone_flash_block);
    let mut button13 = ButtonRp::new(p.PIN_13, PressedTo::Ground);

    let wifi_auto = WifiAutoRp::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH0,
        wifi_credentials_flash_block,
        "DeviceEnvoyClock",
        [timezone_field],
        spawner,
    )?;

    let stack = wifi_auto
        .connect(
            &mut button13,
            async |wifi_auto_event| -> Result<(), device_envoy_rp::Error> {
                match wifi_auto_event {
                    WifiAutoEvent::CaptivePortalReady => {
                        info!("Captive portal ready; connect to DeviceEnvoyClock");
                    }
                    WifiAutoEvent::Connecting {
                        try_index,
                        try_count,
                    } => {
                        info!(
                            "Connecting to Wi-Fi (attempt {} of {})",
                            try_index + 1,
                            try_count
                        );
                    }
                    WifiAutoEvent::ConnectionFailed => {
                        info!("Wi-Fi connection failed");
                    }
                }
                Ok(())
            },
        )
        .await?;

    let timezone_offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    static CLOCK_SYNC_STATIC: ClockSyncStaticRp = ClockSyncRp::new_static();
    let clock_sync = ClockSyncRp::new(
        &CLOCK_SYNC_STATIC,
        stack,
        timezone_offset_minutes,
        Some(ONE_MINUTE),
        spawner,
    )?;

    info!("Wi-Fi connected; logging the time every minute");

    loop {
        let tick = clock_sync.wait_for_tick().await;
        let local_time = tick.local_time;
        info!(
            "Current time: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            local_time.year(),
            u8::from(local_time.month()),
            local_time.day(),
            local_time.hour(),
            local_time.minute(),
            local_time.second(),
        );
    }
}
