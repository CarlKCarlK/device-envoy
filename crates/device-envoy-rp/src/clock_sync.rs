//! A device abstraction that combines NTP time synchronization with a local clock.
//!
//! See [`ClockSync`] for clock operations and [`ClockSyncRp`] for constructors.
//!
//! # Example: WiFi + ClockSync logging
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use defmt_rtt as _;
//! # use panic_probe as _;
//! use device_envoy_rp::{
//!     Error,
//!     Result,
//!     button::{ButtonRp, PressedTo},
//!     clock_sync::{ClockSync as _, ClockSyncRp, ClockSyncStatic, ONE_SECOND, h12_m_s},
//!     flash_block::FlashBlockRp,
//!     wifi_auto::fields::{TimezoneField, TimezoneFieldStatic},
//!     wifi_auto::{WifiAuto as _, WifiAutoEvent, WifiAutoRp},
//! };
//! use defmt::info;
//!
//! async fn run(
//!     spawner: embassy_executor::Spawner,
//!     p: embassy_rp::Peripherals,
//! ) -> Result<(), device_envoy_rp::Error> {
//!     let [wifi_credentials_flash_block, timezone_flash_block] = FlashBlockRp::new_array::<2>(p.FLASH)?;
//!
//!     static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
//!     let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash_block);
//!     let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
//!
//!     let wifi_auto = WifiAutoRp::new(
//!         p.PIN_23,
//!         p.PIN_24,
//!         p.PIN_25,
//!         p.PIN_29,
//!         p.PIO0,
//!         p.DMA_CH0,
//!         wifi_credentials_flash_block,
//!         "ClockSync",
//!         [timezone_field],
//!         spawner,
//!     )?;
//!
//!     let stack = wifi_auto
//!         .connect(&mut button, |event| async move {
//!             match event {
//!                 WifiAutoEvent::CaptivePortalReady => {
//!                     info!("WifiAutoRp: setup mode ready");
//!                 }
//!                 WifiAutoEvent::Connecting { .. } => {
//!                     info!("WifiAutoRp: connecting");
//!                 }
//!                 WifiAutoEvent::ConnectionFailed => {
//!                     info!("WifiAutoRp: connection failed");
//!                 }
//!             }
//!             Ok(())
//!         })
//!         .await?;
//!
//!     let offset_minutes = timezone_field
//!         .offset_minutes()?
//!         .ok_or(Error::MissingCustomWifiAutoField)?;
//!     static CLOCK_SYNC_STATIC: ClockSyncStatic = ClockSyncRp::new_static();
//!     let clock_sync = ClockSyncRp::new(
//!         &CLOCK_SYNC_STATIC,
//!         stack,
//!         offset_minutes,
//!         Some(ONE_SECOND),
//!         spawner,
//!     );
//!
//!     loop {
//!         let tick = clock_sync.wait_for_tick().await;
//!         let (hours, minutes, seconds) = h12_m_s(&tick.local_time);
//!         info!(
//!             "Time {:02}:{:02}:{:02}, since sync {}s",
//!             hours,
//!             minutes,
//!             seconds,
//!             tick.since_last_sync.as_secs()
//!         );
//!     }
//! }
//! ```

#![cfg(feature = "wifi")]

pub use device_envoy_core::clock_sync::{
    ClockSync, ClockSyncRuntime as ClockSyncRp, ClockSyncStatic, ClockSyncTick, ONE_DAY,
    ONE_MINUTE, ONE_SECOND, UnixSeconds, h12_m_s,
};
