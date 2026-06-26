//! A device abstraction for extra setup fields used by [`WifiAutoEsp`](crate::wifi_auto::WifiAutoEsp).
//!
//! See the [`WifiAutoEsp` struct example](crate::wifi_auto::WifiAutoEsp) for the full setup.
//!
//! This module provides ready-to-use field types that can be passed to
//! [`WifiAutoEsp::new`](crate::wifi_auto::WifiAutoEsp::new) for collecting additional
//! configuration beyond WiFi credentials.
//!
//! There are two levels of customization:
//!
//! 1. Use built-in helpers like [`TextField`] and [`TimezoneField`].
//! 2. Define your own field type by implementing [`WifiAutoField`]. See example there.
//!
//! # Example
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! use device_envoy_esp::{
//!     Error, Result,
//!     button::{ButtonEsp, PressedTo},
//!     flash_block::FlashBlockEsp,
//!     wifi_auto::{WifiAuto as _, WifiAutoEvent, WifiAutoEsp},
//!     wifi_auto::fields::{TextField, TextFieldStatic, TimezoneField, TimezoneFieldStatic},
//! };
//!
//! async fn example(
//!     spawner: embassy_executor::Spawner,
//!     p: esp_hal::peripherals::Peripherals,
//! ) -> Result<()> {
//!     let mut button6 = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
//!     let [wifi_flash, website_flash, timezone_flash] = FlashBlockEsp::new_array::<3>(p.FLASH)?;
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
//!     let wifi_auto = WifiAutoEsp::new(
//!         p.WIFI,
//!         wifi_flash,
//!         "DeviceEnvoySetup",
//!         [website_field, timezone_field],
//!         spawner,
//!     )?;
//!
//!     let _stack = wifi_auto
//!         .connect(&mut button6, async |wifi_auto_event| -> Result<(), device_envoy_esp::Error> {
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
#[cfg(target_os = "none")]
use crate::flash_block::FlashBlockEsp;
use device_envoy_core::__impl_wifi_auto_fields;
use device_envoy_core::wifi_auto::{FormData, HtmlBuffer, WifiAutoField};

__impl_wifi_auto_fields!(
    flash_block = FlashBlockEsp,
    error = Error,
    wifi_auto_field = WifiAutoField,
    form_data = FormData<'_>,
    html_buffer = HtmlBuffer,
    flash_cfg = target_os = "none"
);

#[cfg(target_os = "none")]
impl TimezoneField {
    /// Initialize a timezone field backed by a flash block.
    ///
    /// See the [WifiAutoEsp struct example](crate::wifi_auto::WifiAutoEsp) for usage.
    pub fn new(
        timezone_field_static: &'static TimezoneFieldStatic,
        timezone_flash_block: FlashBlockEsp,
    ) -> &'static Self {
        Self::new_with_flash(timezone_field_static, timezone_flash_block)
    }
}

#[cfg(not(target_os = "none"))]
impl TimezoneField {
    /// Initialize a timezone field backed by in-memory state.
    ///
    /// See the [WifiAutoEsp struct example](crate::wifi_auto::WifiAutoEsp) for usage.
    pub fn new(timezone_field_static: &'static TimezoneFieldStatic) -> &'static Self {
        Self::new_in_memory(timezone_field_static)
    }
}

#[cfg(target_os = "none")]
impl<const N: usize> TextField<N> {
    /// Initialize a text field backed by a flash block.
    ///
    /// See the [WifiAutoEsp struct example](crate::wifi_auto::WifiAutoEsp) for usage.
    pub fn new(
        text_field_static: &'static TextFieldStatic<N>,
        text_flash_block: FlashBlockEsp,
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

#[cfg(not(target_os = "none"))]
impl<const N: usize> TextField<N> {
    /// Initialize a text field backed by in-memory state.
    ///
    /// See the [WifiAutoEsp struct example](crate::wifi_auto::WifiAutoEsp) for usage.
    pub fn new(
        text_field_static: &'static TextFieldStatic<N>,
        field_name: &'static str,
        label: &'static str,
        default_value: &'static str,
    ) -> &'static Self {
        Self::new_in_memory(text_field_static, field_name, label, default_value)
    }
}
