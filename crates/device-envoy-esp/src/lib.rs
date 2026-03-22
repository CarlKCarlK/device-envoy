#![doc = include_str!("../README.md")]
#![cfg_attr(target_os = "none", no_std)]

#[cfg(all(
    target_os = "none",
    not(any(
        feature = "esp32",
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s2",
        feature = "esp32s3"
    ))
))]
compile_error!(
    "Select one chip feature for embedded builds: `esp32`, `esp32c2`, `esp32c3`, `esp32c6`, `esp32h2`, `esp32s2`, or `esp32s3` (with `--no-default-features --features <chip>`)."
);

#[cfg(all(
    target_os = "none",
    any(
        all(feature = "esp32", feature = "esp32c2"),
        all(feature = "esp32", feature = "esp32c3"),
        all(feature = "esp32", feature = "esp32c6"),
        all(feature = "esp32", feature = "esp32h2"),
        all(feature = "esp32", feature = "esp32s2"),
        all(feature = "esp32", feature = "esp32s3"),
        all(feature = "esp32c2", feature = "esp32c3"),
        all(feature = "esp32c2", feature = "esp32c6"),
        all(feature = "esp32c2", feature = "esp32h2"),
        all(feature = "esp32c2", feature = "esp32s2"),
        all(feature = "esp32c2", feature = "esp32s3"),
        all(feature = "esp32c3", feature = "esp32c6"),
        all(feature = "esp32c3", feature = "esp32h2"),
        all(feature = "esp32c3", feature = "esp32s2"),
        all(feature = "esp32c3", feature = "esp32s3"),
        all(feature = "esp32c6", feature = "esp32h2"),
        all(feature = "esp32c6", feature = "esp32s2"),
        all(feature = "esp32c6", feature = "esp32s3"),
        all(feature = "esp32h2", feature = "esp32s2"),
        all(feature = "esp32h2", feature = "esp32s3"),
        all(feature = "esp32s2", feature = "esp32s3"),
    )
))]
compile_error!("Select exactly one chip feature for embedded builds, not both.");

pub mod button;
#[cfg(all(target_os = "none", esp_has_wifi))]
pub mod clock_sync {
    //! A device abstraction that combines NTP time synchronization with a local clock.
    //!
    //! See [`ClockSyncEsp`] for constructors and [`ClockSync`] for clock operations.
    //!
    //! You can create up to two concurrent `ClockSyncEsp` instances per program; a third is expected to fail at runtime because the `clock_sync` task pool uses `pool_size = 2`.
    //!
    //! # Example: WiFi + ClockSync logging
    //!
    //! ```rust,no_run
    //! # #![no_std]
    //! # #![no_main]
    //! use device_envoy_esp::{
    //!     Error,
    //!     Result,
    //!     button::PressedTo,
    //!     button_watch,
    //!     clock_sync::{ClockSync as _, ClockSyncEsp, ClockSyncStaticEsp, ONE_SECOND, h12_m_s},
    //!     flash_block::FlashBlockEsp,
    //!     wifi_auto::{
    //!         WifiAuto as _, WifiAutoEsp, WifiAutoEvent,
    //!         fields::{TimezoneField, TimezoneFieldStatic},
    //!     },
    //! };
    //! use log::info;
    //!
    //! button_watch! {
    //!     ButtonWatch6 {
    //!         pin: GPIO6,
    //!     }
    //! }
    //!
    //! async fn run(
    //!     spawner: embassy_executor::Spawner,
    //!     p: esp_hal::peripherals::Peripherals,
    //! ) -> Result<()> {
    //!     let [wifi_credentials_flash_block, timezone_flash_block] =
    //!         FlashBlockEsp::new_array::<2>(p.FLASH)?;
    //!
    //!     static TIMEZONE_STATIC: TimezoneFieldStatic = TimezoneField::new_static();
    //!     let timezone_field = TimezoneField::new(&TIMEZONE_STATIC, timezone_flash_block);
    //!
    //!     let button_watch6 = ButtonWatch6::new(p.GPIO6, PressedTo::Ground, spawner).await?;
    //!     let wifi_auto = WifiAutoEsp::new(
    //!         p.WIFI,
    //!         wifi_credentials_flash_block,
    //!         "ClockSync",
    //!         [timezone_field],
    //!         spawner,
    //!     )?;
    //!
    //!     let stack = wifi_auto
    //!         .connect(&mut *button_watch6, |event| async move {
    //!             match event {
    //!                 WifiAutoEvent::CaptivePortalReady => {
    //!                     info!("WifiAutoEsp: setup mode ready");
    //!                 }
    //!                 WifiAutoEvent::Connecting { .. } => {
    //!                     info!("WifiAutoEsp: connecting");
    //!                 }
    //!                 WifiAutoEvent::ConnectionFailed => {
    //!                     info!("WifiAutoEsp: connection failed");
    //!                 }
    //!             }
    //!             Ok(())
    //!         })
    //!         .await?;
    //!
    //!     let offset_minutes = timezone_field
    //!         .offset_minutes()?
    //!         .ok_or(Error::MissingCustomWifiAutoField)?;
    //!     static CLOCK_SYNC_STATIC: ClockSyncStaticEsp = ClockSyncEsp::new_static();
    //!     let clock_sync = ClockSyncEsp::new(
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
    /// A device abstraction that combines NTP time synchronization with a local clock.
    pub use device_envoy_core::clock_sync::ClockSyncRuntime as ClockSyncEsp;
    /// Resources needed to construct [`ClockSyncEsp`].
    pub use device_envoy_core::clock_sync::ClockSyncStatic as ClockSyncStaticEsp;
    pub use device_envoy_core::clock_sync::{
        h12_m_s, ClockSync, ClockSyncTick, UnixSeconds, ONE_DAY, ONE_MINUTE, ONE_SECOND,
    };
}
#[cfg(all(target_os = "none", esp_has_wifi))]
#[doc(hidden)]
pub mod time_sync {
    //! A device abstraction for Network Time Protocol (NTP) time synchronization over Wi-Fi.
    //! See the [`clock_sync` module](crate::clock_sync) for the high-level clock API.
    pub use device_envoy_core::clock_sync::UnixSeconds;
    pub use device_envoy_core::time_sync::{TimeSync, TimeSyncEvent, TimeSyncStatic};
}
#[cfg(esp_has_i2s)]
pub mod audio_player;
pub mod flash_block;
pub mod init_and_start;
#[cfg(esp_has_rmt)]
pub mod ir;
#[cfg(target_os = "none")]
pub mod lcd_text;
#[cfg(target_os = "none")]
pub mod led;
#[cfg(any(feature = "host", esp_has_rmt))]
pub mod led2d;
pub mod led4;
#[cfg(esp_has_rmt)]
pub mod led_strip;
#[cfg(target_os = "none")]
pub mod rfid;
#[cfg(esp_has_rmt)]
mod rmt;
mod rmt_mode;
#[cfg(target_os = "none")]
pub mod servo;
#[cfg(target_os = "none")]
mod servo_player;
#[cfg(esp_has_wifi)]
pub mod wifi_auto;

#[cfg(doc)]
pub mod docs {
    //! Documentation-only pages for this crate.
    pub mod development_guide {
        #![doc = include_str!("docs/development_guide.md")]
    }
}

pub use device_envoy_core::tone;
#[cfg(esp_has_wifi)]
use device_envoy_core::wifi_auto::WifiAutoError;
/// Used internally by other macros.
#[doc(hidden)]
pub use paste::paste as __paste;

/// Public for macro expansion in downstream crates.
#[doc(hidden)]
#[macro_export]
macro_rules! __validate_keyword_fields_expr {
    (
        macro_name: $macro_name:literal,
        allowed_macro: $allowed_macro:path,
        fields: [ $( $field:ident : $value:expr ),* $(,)? ]
    ) => {
        const _: () = {
            $( $allowed_macro!($field, $macro_name); )*
            #[allow(non_snake_case)]
            mod __device_envoy_keyword_fields_uniqueness {
                $( pub(super) mod $field {} )*
            }
        };
    };

    (
        macro_name: $macro_name:literal,
        allowed_macro: $allowed_macro:path,
        fields: [ $($fields:tt)* ]
    ) => {
        compile_error!(concat!($macro_name, " fields must use `name: value` syntax"));
    };
}

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
    #[cfg(all(target_os = "none", esp_has_rmt))]
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
    #[cfg(all(target_os = "none", esp_has_wifi))]
    WifiInit(esp_radio::InitializationError),
    #[cfg(all(target_os = "none", esp_has_wifi))]
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

#[cfg(esp_has_wifi)]
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

#[cfg(all(target_os = "none", esp_has_wifi))]
impl From<esp_radio::InitializationError> for Error {
    fn from(error: esp_radio::InitializationError) -> Self {
        Self::WifiInit(error)
    }
}

#[cfg(all(target_os = "none", esp_has_rmt))]
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

#[cfg(all(target_os = "none", esp_has_wifi))]
impl From<esp_radio::wifi::WifiError> for Error {
    fn from(error: esp_radio::wifi::WifiError) -> Self {
        Self::Wifi(error)
    }
}
