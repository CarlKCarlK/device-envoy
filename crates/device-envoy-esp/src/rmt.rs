//! A device abstraction for shared ESP32 RMT initialization and channel configuration.
//!
//! See [`Rmt80`] and [`new_rmt80`] for the shared 80 MHz RMT entry point.
#![cfg_attr(not(target_os = "none"), allow(dead_code))]

#[cfg(target_os = "none")]
use crate::Result;

#[cfg(target_os = "none")]
pub type Rmt80<'rmt> = esp_hal::rmt::Rmt<'rmt, esp_hal::Blocking>;

#[cfg(target_os = "none")]
pub type Rmt80Async<'rmt> = esp_hal::rmt::Rmt<'rmt, esp_hal::Async>;

/// Project default RMT source clock in MHz.
pub const RMT_RATE_MHZ: u32 = 80;

/// Create the shared RMT hub at 80 MHz.
///
/// This is the single initialization path for RMT in this crate.
#[cfg(target_os = "none")]
pub fn new_rmt80(rmt: esp_hal::peripherals::RMT<'static>) -> Result<Rmt80<'static>> {
    let rmt80 = esp_hal::rmt::Rmt::new(rmt, esp_hal::time::Rate::from_mhz(RMT_RATE_MHZ))
        .map_err(crate::Error::RmtConfig)?;
    Ok(rmt80)
}

/// Convert a shared blocking RMT hub to async mode.
#[cfg(target_os = "none")]
#[must_use]
pub fn into_async(rmt80: Rmt80<'static>) -> Rmt80Async<'static> {
    rmt80.into_async()
}

/// Standard TX channel config for NeoPixel-style (WS2812) output.
#[cfg(target_os = "none")]
#[must_use]
pub fn ws2812_tx_config() -> esp_hal::rmt::TxChannelConfig {
    esp_hal::rmt::TxChannelConfig::default()
        .with_clk_divider(4)
        .with_idle_output_level(esp_hal::gpio::Level::Low)
        .with_idle_output(true)
        .with_carrier_modulation(false)
}

/// Standard RX channel config for NEC IR decoding with 1 us ticks.
#[cfg(target_os = "none")]
#[must_use]
pub fn nec_rx_config() -> esp_hal::rmt::RxChannelConfig {
    esp_hal::rmt::RxChannelConfig::default()
        .with_clk_divider(80)
        .with_filter_threshold(20)
        .with_idle_threshold(12_000)
}
