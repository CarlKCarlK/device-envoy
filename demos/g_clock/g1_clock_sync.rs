#![allow(missing_docs)]
//! WiFi + ClockSync demo that logs time and sync age every second.

#![no_std]
#![no_main]
#![cfg(feature = "wifi")]
#![allow(clippy::future_not_send, reason = "single-threaded")]

use core::{convert::Infallible, panic};
use defmt::info;
use device_kit::{
    Error, Result,
    button::PressedTo,
    clock_sync::{ClockSync, ClockSyncStatic, ONE_SECOND, h12_m_s},
    flash_array::{FlashArray, FlashArrayStatic},
    wifi_auto::fields::{TimezoneField, TimezoneFieldStatic},
    wifi_auto::{WifiAuto, WifiAutoEvent},
};
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err}");
}

async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());

    // We're going to store two blocks in flash: WiFi credentials and timezone.
    static FLASH_STATIC: FlashArrayStatic = FlashArray::<2>::new_static();
    let [wifi_credentials_flash_block, timezone_flash_block] =
        FlashArray::new(&FLASH_STATIC, p.FLASH)?;

    // Timezone is an optional field that users can set on the setup website.
    static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash_block);

    let wifi_auto = WifiAuto::new(
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH0,
        wifi_credentials_flash_block,
        p.PIN_13,
        PressedTo::Ground,
        "ClockSync",
        [timezone_field], // Additional field(s)
        spawner,
    )?;

    let (stack, _button) = wifi_auto
        .connect(|event| async move {
            match event {
                WifiAutoEvent::CaptivePortalReady => {
                    info!("WifiAuto: setup mode ready");
                }
                WifiAutoEvent::Connecting { .. } => {
                    info!("WifiAuto: connecting");
                }
                WifiAutoEvent::ConnectionFailed => {
                    info!("WifiAuto: connection failed");
                }
            }
            Ok(())
        })
        .await?;

    // Extract the timezone offset (in minutes) from the timezone field.
    let offset_minutes = timezone_field
        .offset_minutes()?
        .ok_or(Error::MissingCustomWifiAutoField)?;

    // Create the ClockSync instance.
    static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSync::new_static();
    let clock_sync = ClockSync::new(
        &CLOCK_SYNC_STATIC,
        stack,
        offset_minutes,
        Some(ONE_SECOND), // Tick at the top of every second
        spawner,
    );

    // Main loop: wait for ticks and log the time and sync age.
    loop {
        let tick = clock_sync.wait_for_tick().await;
        let (hours, minutes, seconds) = h12_m_s(&tick.local_time);
        info!(
            "Time {:02}:{:02}:{:02}, since sync {}s",
            hours,
            minutes,
            seconds,
            tick.since_last_sync.as_secs()
        );
    }
}
