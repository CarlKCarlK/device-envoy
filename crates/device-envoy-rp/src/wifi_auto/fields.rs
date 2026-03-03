//! A device abstraction for extra setup fields used by [`crate::wifi_auto::WifiAuto`].
//! See the [`WifiAuto` struct example](crate::wifi_auto::WifiAuto) for the full setup.

#![allow(
    unsafe_code,
    reason = "unsafe impl Sync is sound: single-threaded Embassy executor, no concurrent access"
)]

use crate::Error;
use crate::flash_array::FlashBlock;
use device_envoy_core::__impl_wifi_auto_fields;
use device_envoy_core::wifi_auto::{FormData, HtmlBuffer};

__impl_wifi_auto_fields!(
    flash_block = FlashBlock,
    error = Error,
    wifi_auto_field = device_envoy_core::wifi_auto::WifiAutoField,
    form_data = FormData<'_>,
    html_buffer = HtmlBuffer
);

impl TimezoneField {
    /// Initialize a timezone field backed by a flash block.
    ///
    /// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
    pub fn new(
        timezone_field_static: &'static TimezoneFieldStatic,
        timezone_flash_block: FlashBlock,
    ) -> &'static Self {
        Self::new_with_flash(timezone_field_static, timezone_flash_block)
    }
}

impl<const N: usize> TextField<N> {
    /// Initialize a text field backed by a flash block.
    ///
    /// See the [WifiAuto struct example](crate::wifi_auto::WifiAuto) for usage.
    pub fn new(
        text_field_static: &'static TextFieldStatic<N>,
        text_flash_block: FlashBlock,
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
