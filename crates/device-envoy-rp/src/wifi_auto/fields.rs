//! A device abstraction for extra setup fields used by [`WifiAutoRp`](crate::wifi_auto::WifiAutoRp).
//!
//! See the [`WifiAutoRp` struct example](crate::wifi_auto::WifiAutoRp) for the full setup.
//!
//! This module provides ready-to-use field types that can be passed to
//! [`WifiAutoRp::new`](crate::wifi_auto::WifiAutoRp::new) for collecting additional
//! configuration beyond WiFi credentials.
//!
//! There are two levels of customization:
//!
//! 1. Use built-in helpers like [`TextField`] and [`TimezoneField`].
//! 2. Define your own field type by implementing [`WifiAutoField`](crate::wifi_auto::WifiAutoField). See example there.
//!
//! # Example
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! use device_envoy_rp::{
//!     Error, Result,
//!     button::PressedTo,
//!     button_watch,
//!     flash_block::FlashBlockRp,
//!     wifi_auto::{WifiAuto as _, WifiAutoEvent, WifiAutoRp},
//!     wifi_auto::fields::{TextField, TextFieldStatic, TimezoneField, TimezoneFieldStatic},
//! };
//!
//! button_watch! {
//!     ButtonWatch13 {
//!         pin: PIN_13,
//!     }
//! }
//!
//! async fn example(
//!     spawner: embassy_executor::Spawner,
//!     p: embassy_rp::Peripherals,
//! ) -> Result<()> {
//!     let [wifi_flash, website_flash, timezone_flash] = FlashBlockRp::new_array::<3>(p.FLASH)?;
//!
//!     static WEBSITE_STATIC: TextFieldStatic<32> = TextField::new_static();
//!     let website_field = TextField::new(
//!         &WEBSITE_STATIC,
//!         website_flash,
//!         "website",
//!         "Website",
//!         "google.com",
//!     );
//!
//!     static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
//!     let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash);
//!
//!     let button_watch13 = ButtonWatch13::new(p.PIN_13, PressedTo::Ground, spawner).await?;
//!     let wifi_auto = WifiAutoRp::new(
//!         p.PIN_23,
//!         p.PIN_24,
//!         p.PIN_25,
//!         p.PIN_29,
//!         p.PIO0,
//!         p.DMA_CH0,
//!         wifi_flash,
//!         "DeviceEnvoySetup",
//!         [website_field, timezone_field],
//!         spawner,
//!     )?;
//!
//!     let _stack = wifi_auto
//!         .connect(&mut *button_watch13, |wifi_auto_event| async move {
//!             match wifi_auto_event {
//!                 WifiAutoEvent::CaptivePortalReady => {}
//!                 WifiAutoEvent::Connecting { .. } => {}
//!                 WifiAutoEvent::ConnectionFailed => {}
//!             }
//!             Ok(())
//!         })
//!         .await?;
//!
//!     let _website = website_field.text()?.unwrap_or_default();
//!     let _offset_minutes = timezone_field
//!         .offset_minutes()?
//!         .ok_or(Error::MissingCustomWifiAutoField)?;
//!
//!     Ok(())
//! }
//! ```

#![allow(
    unsafe_code,
    reason = "unsafe impl Sync is sound: single-threaded Embassy executor, no concurrent access"
)]

use crate::Error;
use crate::flash_block::FlashBlockRp;
use device_envoy_core::__impl_wifi_auto_fields;
use device_envoy_core::wifi_auto::{FormData, HtmlBuffer};

__impl_wifi_auto_fields!(
    flash_block = FlashBlockRp,
    error = Error,
    wifi_auto_field = device_envoy_core::wifi_auto::WifiAutoField,
    form_data = FormData<'_>,
    html_buffer = HtmlBuffer
);

impl TimezoneField {
    /// Initialize a timezone field backed by a flash block.
    ///
    /// See the [WifiAutoRp struct example](crate::wifi_auto::WifiAutoRp) for usage.
    pub fn new(
        timezone_field_static: &'static TimezoneFieldStatic,
        timezone_flash_block: FlashBlockRp,
    ) -> &'static Self {
        Self::new_with_flash(timezone_field_static, timezone_flash_block)
    }
}

impl<const N: usize> TextField<N> {
    /// Initialize a text field backed by a flash block.
    ///
    /// See the [WifiAutoRp struct example](crate::wifi_auto::WifiAutoRp) for usage.
    pub fn new(
        text_field_static: &'static TextFieldStatic<N>,
        text_flash_block: FlashBlockRp,
        field_name: &'static str,
        label: &'static str,
        default_value: &'static str,
    ) -> &'static Self {
        Self::new_with_flash(
            text_field_static,
            text_flash_block,
            field_name,
            label,
            default_value,
        )
    }
}
