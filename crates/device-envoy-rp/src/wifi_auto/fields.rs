//! A device abstraction for extra setup fields used by [`crate::wifi_auto::WifiAutoRp`].
//! See the [`WifiAutoRp` struct example](crate::wifi_auto::WifiAutoRp) for the full setup.

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
