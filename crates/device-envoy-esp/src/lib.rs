//! device-envoy-esp — NeoPixel-style (WS2812) LED strips and panels for ESP32.
//!
//! See the [`led_strip` module](mod@led_strip) and [`led2d` module](mod@led2d).

#![cfg_attr(target_os = "none", no_std)]

pub mod button;
#[cfg(target_os = "none")]
pub(crate) mod clock;
#[cfg(target_os = "none")]
pub mod clock_sync;
pub mod audio_player;
pub mod flash_array;
pub mod ir;
pub mod led2d;
pub mod led_strip;
pub mod rmt;
pub mod rmt_mode;
#[cfg(target_os = "none")]
pub mod time_sync;
pub mod wifi_auto;

pub use led_strip::{colors, Frame1d, Gamma, ToRgb8, ToRgb888, RGB8};

#[doc(hidden)]
#[cfg(target_os = "none")]
pub use esp_hal;
#[doc(hidden)]
#[cfg(target_os = "none")]
pub use esp_rtos;

#[cfg(target_os = "none")]
#[macro_export]
macro_rules! init_and_start {
    ($p:ident, $rmt80:ident, rmt_mode::Blocking) => {
        $crate::init_and_start!($p);
        let $rmt80 = $crate::rmt::new_rmt80($p.RMT).expect("RMT init failed");
    };
    ($p:ident, $rmt80:ident, rmt_mode::Async) => {
        $crate::init_and_start!($p);
        let $rmt80 =
            $crate::rmt::into_async($crate::rmt::new_rmt80($p.RMT).expect("RMT init failed"));
    };
    ($p:ident, $rmt80:ident) => {
        compile_error!(
            "init_and_start!(p, rmt80) now requires mode: init_and_start!(p, rmt80, rmt_mode::Blocking|Async)"
        );
    };
    ($p:ident) => {
        let $p = $crate::esp_hal::init($crate::esp_hal::Config::default());
        {
            let timg0 = $crate::esp_hal::timer::timg::TimerGroup::new($p.TIMG0);
            #[cfg(target_arch = "riscv32")]
            {
                let sw = $crate::esp_hal::interrupt::software::SoftwareInterruptControl::new(
                    $p.SW_INTERRUPT,
                );
                $crate::esp_rtos::start(timg0.timer0, sw.software_interrupt0);
            }
            #[cfg(target_arch = "xtensa")]
            {
                $crate::esp_rtos::start(timg0.timer0);
            }
        }
    };
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    TaskSpawn(embassy_executor::SpawnError),
    #[cfg(target_os = "none")]
    FlashStorage(esp_storage::FlashStorageError),
    InvalidFlashRegion,
    IndexOutOfBounds,
    FormatError,
    StorageCorrupted,
    FlashRegionMismatch,
    MissingCustomWifiAutoField,
    Ntp(&'static str),
    #[cfg(target_os = "none")]
    Rmt(esp_hal::rmt::Error),
    #[cfg(target_os = "none")]
    SpiConfig(esp_hal::spi::master::ConfigError),
    #[cfg(target_os = "none")]
    Spi(esp_hal::spi::Error),
    #[cfg(target_os = "none")]
    WifiInit(esp_radio::InitializationError),
    #[cfg(target_os = "none")]
    Wifi(esp_radio::wifi::WifiError),
}

impl From<embassy_executor::SpawnError> for Error {
    fn from(e: embassy_executor::SpawnError) -> Self {
        Self::TaskSpawn(e)
    }
}

#[cfg(target_os = "none")]
impl From<esp_storage::FlashStorageError> for Error {
    fn from(error: esp_storage::FlashStorageError) -> Self {
        Self::FlashStorage(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_radio::InitializationError> for Error {
    fn from(error: esp_radio::InitializationError) -> Self {
        Self::WifiInit(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::rmt::Error> for Error {
    fn from(error: esp_hal::rmt::Error) -> Self {
        Self::Rmt(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::spi::master::ConfigError> for Error {
    fn from(error: esp_hal::spi::master::ConfigError) -> Self {
        Self::SpiConfig(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::spi::Error> for Error {
    fn from(error: esp_hal::spi::Error) -> Self {
        Self::Spi(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_radio::wifi::WifiError> for Error {
    fn from(error: esp_radio::wifi::WifiError) -> Self {
        Self::Wifi(error)
    }
}
