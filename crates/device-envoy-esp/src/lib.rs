#![doc = include_str!("../README.md")]
// todo00 does led_strip for esp have two methods that rp doesn't?
// todo00 check if every async method should be async?
// todo00 rename check-all to check-rp and check-esp? likewise attach
#![cfg_attr(target_os = "none", no_std)]

pub mod button;
#[cfg(target_os = "none")]
pub mod clock_sync {
    //! A device abstraction that combines NTP time synchronization with a local clock.
    //! See [`ClockSync`] for clock operations and [`ClockSyncEsp`] for constructors.
    pub use device_envoy_core::clock_sync::{
        h12_m_s, ClockSync, ClockSyncRuntime as ClockSyncEsp, ClockSyncStatic, ClockSyncTick,
        UnixSeconds, ONE_DAY, ONE_MINUTE, ONE_SECOND,
    };
}
#[cfg(target_os = "none")]
pub mod time_sync {
    //! A device abstraction for Network Time Protocol (NTP) time synchronization over Wi-Fi.
    //! See the [`clock_sync` module](crate::clock_sync) for the high-level clock API.
    pub use device_envoy_core::clock::UnixSeconds;
    pub use device_envoy_core::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic};
}
pub mod audio_player;
pub mod flash_block;
pub mod init_and_start;
pub mod ir;
#[cfg(target_os = "none")]
pub mod lcd_text;
#[cfg(target_os = "none")]
pub mod led;
pub mod led2d;
pub mod led4;
pub mod led_strip;
#[cfg(target_os = "none")]
pub mod rfid;
mod rmt;
mod rmt_mode;
#[cfg(target_os = "none")]
pub mod servo;
#[cfg(target_os = "none")]
mod servo_player;
pub mod wifi_auto;

#[cfg(doc)]
pub mod docs {
    //! Documentation-only pages for this crate.
    #[doc = include_str!("../../device-envoy-core/docs/development.md")]
    pub mod development_guide {}
}

pub use device_envoy_core::tone;
use device_envoy_core::wifi_auto::WifiAutoError;
/// Used internally by other macros.
#[doc(hidden)]
pub use paste::paste as __paste;

// Workaround for esp-radio 0.17 bug: the linker script for esp32c6 declares EXTERN for
// __esp_radio_misc_nvs_init and __esp_radio_misc_nvs_deinit under the wifi section, but
// esp-radio only defines them with #[cfg(xtensa)], leaving RISC-V targets with unresolved
// symbols in release builds.  These no-op stubs reproduce exactly what the Xtensa
// implementation does.  Remove this block when the upstream bug is fixed.
//
// SAFETY: `no_mangle` is required because the linker script demands these exact C symbol
// names.  The functions are no-ops that match the Xtensa stubs in esp-radio's
// common_adapter.rs; they are called by the wifi blob and must have C linkage.
#[cfg(all(target_arch = "riscv32", target_os = "none"))]
mod _esp_radio_nvs_stubs {
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __esp_radio_misc_nvs_deinit() {}

    #[unsafe(no_mangle)]
    unsafe extern "C" fn __esp_radio_misc_nvs_init() -> i32 {
        0
    }
}

#[doc(hidden)]
#[cfg(target_os = "none")]
pub use esp_hal;
#[doc(hidden)]
#[cfg(target_os = "none")]
pub use esp_rtos;

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
    Led4BitsToIndexesFull,
    MissingCustomWifiAutoField,
    Ntp(&'static str),
    #[cfg(target_os = "none")]
    Rmt(esp_hal::rmt::Error),
    #[cfg(target_os = "none")]
    SpiConfig(esp_hal::spi::master::ConfigError),
    #[cfg(target_os = "none")]
    Spi(esp_hal::spi::Error),
    #[cfg(target_os = "none")]
    Mfrc522Init(esp_hal_mfrc522::consts::PCDErrorCode),
    #[cfg(target_os = "none")]
    Mfrc522Version(esp_hal_mfrc522::consts::PCDErrorCode),
    #[cfg(target_os = "none")]
    I2cConfig(esp_hal::i2c::master::ConfigError),
    #[cfg(target_os = "none")]
    LedcTimer(esp_hal::ledc::timer::Error),
    #[cfg(target_os = "none")]
    LedcChannel(esp_hal::ledc::channel::Error),
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

impl From<device_envoy_core::led4::Led4BitsToIndexesError> for Error {
    fn from(error: device_envoy_core::led4::Led4BitsToIndexesError) -> Self {
        match error {
            device_envoy_core::led4::Led4BitsToIndexesError::Full => Self::Led4BitsToIndexesFull,
        }
    }
}

impl From<WifiAutoError> for Error {
    fn from(error: WifiAutoError) -> Self {
        match error {
            WifiAutoError::FormatError => Self::FormatError,
            WifiAutoError::StorageCorrupted => Self::StorageCorrupted,
            WifiAutoError::MissingCustomWifiAutoField => Self::MissingCustomWifiAutoField,
        }
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
impl From<esp_hal::i2c::master::ConfigError> for Error {
    fn from(error: esp_hal::i2c::master::ConfigError) -> Self {
        Self::I2cConfig(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::spi::Error> for Error {
    fn from(error: esp_hal::spi::Error) -> Self {
        Self::Spi(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::ledc::timer::Error> for Error {
    fn from(error: esp_hal::ledc::timer::Error) -> Self {
        Self::LedcTimer(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_hal::ledc::channel::Error> for Error {
    fn from(error: esp_hal::ledc::channel::Error) -> Self {
        Self::LedcChannel(error)
    }
}

#[cfg(target_os = "none")]
impl From<esp_radio::wifi::WifiError> for Error {
    fn from(error: esp_radio::wifi::WifiError) -> Self {
        Self::Wifi(error)
    }
}
