#![doc = include_str!("../README.md")]
#![cfg_attr(target_os = "none", no_std)]

#[cfg(all(
    target_os = "none",
    not(any(
        feature = "esp32",
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32c5",
        feature = "esp32c6",
        feature = "esp32c61",
        feature = "esp32h2",
        feature = "esp32s2",
        feature = "esp32s3"
    ))
))]
compile_error!(
    "Select one chip feature for embedded builds: `esp32`, `esp32c2`, `esp32c3`, `esp32c5`, `esp32c6`, `esp32c61`, `esp32h2`, `esp32s2`, or `esp32s3` (with `--no-default-features --features <chip>`)."
);

#[cfg(all(
    target_os = "none",
    any(
        all(feature = "esp32", feature = "esp32c2"),
        all(feature = "esp32", feature = "esp32c3"),
        all(feature = "esp32", feature = "esp32c5"),
        all(feature = "esp32", feature = "esp32c6"),
        all(feature = "esp32", feature = "esp32c61"),
        all(feature = "esp32", feature = "esp32h2"),
        all(feature = "esp32", feature = "esp32s2"),
        all(feature = "esp32", feature = "esp32s3"),
        all(feature = "esp32c2", feature = "esp32c3"),
        all(feature = "esp32c2", feature = "esp32c5"),
        all(feature = "esp32c2", feature = "esp32c6"),
        all(feature = "esp32c2", feature = "esp32c61"),
        all(feature = "esp32c2", feature = "esp32h2"),
        all(feature = "esp32c2", feature = "esp32s2"),
        all(feature = "esp32c2", feature = "esp32s3"),
        all(feature = "esp32c3", feature = "esp32c5"),
        all(feature = "esp32c3", feature = "esp32c6"),
        all(feature = "esp32c3", feature = "esp32c61"),
        all(feature = "esp32c3", feature = "esp32h2"),
        all(feature = "esp32c3", feature = "esp32s2"),
        all(feature = "esp32c3", feature = "esp32s3"),
        all(feature = "esp32c5", feature = "esp32c6"),
        all(feature = "esp32c5", feature = "esp32c61"),
        all(feature = "esp32c5", feature = "esp32h2"),
        all(feature = "esp32c5", feature = "esp32s2"),
        all(feature = "esp32c5", feature = "esp32s3"),
        all(feature = "esp32c6", feature = "esp32h2"),
        all(feature = "esp32c6", feature = "esp32c61"),
        all(feature = "esp32c6", feature = "esp32s2"),
        all(feature = "esp32c6", feature = "esp32s3"),
        all(feature = "esp32c61", feature = "esp32h2"),
        all(feature = "esp32c61", feature = "esp32s2"),
        all(feature = "esp32c61", feature = "esp32s3"),
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
    //! Constructor methods on `ClockSyncEsp` come from `device-envoy-core` and return
    //! [`CoreResult`] with [`CoreError`].
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
    //!         .connect(&mut *button_watch6, async |event| -> Result<(), device_envoy_esp::Error> {
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
    //!     )?;
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
        ClockSync, ClockSyncTick, ONE_DAY, ONE_MINUTE, ONE_SECOND, UnixSeconds, h12_m_s,
    };
    pub use device_envoy_core::{Error as CoreError, Result as CoreResult};
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
#[cfg(target_os = "none")]
pub mod cyd;
pub mod flash_block;
pub mod init_and_start;
#[cfg(esp_has_rmt)]
pub mod ir;
#[cfg(target_os = "none")]
pub mod lcd_text;
#[cfg(target_os = "none")]
pub mod led;
#[cfg(any(feature = "host", target_os = "none"))]
pub mod led2d;
pub mod led4;
#[cfg(target_os = "none")]
pub mod led_strip;
#[cfg(target_os = "none")]
pub mod rfid;
#[cfg(esp_has_rmt)]
mod rmt;
mod rmt_mode;
#[cfg(all(target_os = "none", esp_has_ledc))]
pub mod servo;
#[cfg(all(target_os = "none", esp_has_ledc))]
mod servo_player;
#[cfg(any(feature = "host", esp_has_wifi))]
pub mod wifi_auto;

#[cfg(doc)]
pub mod docs {
    //! Documentation-only pages for this crate.
    pub mod development_guide {
        #![doc = include_str!("docs/development_guide.md")]
    }
}

pub use device_envoy_core::tone;
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

#[derive(Debug, derive_more::From)]
#[non_exhaustive]
pub enum Error {
    TaskSpawn(embassy_executor::SpawnError),
    #[from(ignore)]
    Core(device_envoy_core::Error),
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
    RmtConfig(esp_hal::rmt::ConfigError),
    #[cfg(all(target_os = "none", esp_has_rmt))]
    Rmt(esp_hal::rmt::Error),
    #[cfg(target_os = "none")]
    SpiConfig(esp_hal::spi::master::ConfigError),
    #[cfg(target_os = "none")]
    Spi(esp_hal::spi::Error),
    #[cfg(target_os = "none")]
    #[from(ignore)]
    Mfrc522Init(esp_hal_mfrc522::consts::PCDErrorCode),
    #[cfg(target_os = "none")]
    #[from(ignore)]
    Mfrc522Version(esp_hal_mfrc522::consts::PCDErrorCode),
    #[cfg(target_os = "none")]
    I2cConfig(esp_hal::i2c::master::ConfigError),
    #[cfg(all(target_os = "none", esp_has_ledc))]
    LedcTimer(esp_hal::ledc::timer::Error),
    #[cfg(all(target_os = "none", esp_has_ledc))]
    LedcChannel(esp_hal::ledc::channel::Error),
    #[cfg(all(target_os = "none", esp_has_wifi))]
    Wifi(esp_radio::wifi::WifiError),
    #[cfg(target_os = "none")]
    CydDisplayInit(cyd::CydDisplayEspInitError),
    #[cfg(target_os = "none")]
    CydTouchInit(cyd::CydTouchEspInitError),
    #[cfg(target_os = "none")]
    #[from(ignore)]
    CydDisplayFlush(cyd::CydDisplayEspFlushError),
    #[cfg(target_os = "none")]
    #[from(ignore)]
    CydDisplaySetOrientation(cyd::CydDisplayEspFlushError),
    #[cfg(target_os = "none")]
    CydTouchUnavailable,
}

#[cfg(target_os = "none")]
impl From<cyd::CydError> for Error {
    fn from(error: cyd::CydError) -> Self {
        match error {
            cyd::CydError::DisplayInit(error) => Self::CydDisplayInit(error),
            cyd::CydError::TouchInit(error) => Self::CydTouchInit(error),
            cyd::CydError::DisplayFlush(error) => Self::CydDisplayFlush(error),
            cyd::CydError::DisplaySetOrientation(error) => Self::CydDisplaySetOrientation(error),
        }
    }
}

#[cfg(target_os = "none")]
impl
    From<
        device_envoy_core::cyd::touch::calibration::EnsureCalibrationError<
            cyd::CydTouchUncalibratedEsp,
            Error,
        >,
    > for Error
{
    fn from(
        error: device_envoy_core::cyd::touch::calibration::EnsureCalibrationError<
            cyd::CydTouchUncalibratedEsp,
            Error,
        >,
    ) -> Self {
        match error.kind {
            device_envoy_core::cyd::touch::calibration::EnsureCalibrationErrorKind::Device(
                error,
            ) => Self::from(error),
            device_envoy_core::cyd::touch::calibration::EnsureCalibrationErrorKind::Flash(
                error,
            ) => error,
        }
    }
}

impl From<device_envoy_core::Error> for Error {
    fn from(error: device_envoy_core::Error) -> Self {
        match error {
            device_envoy_core::Error::TaskSpawn(spawn_error) => Self::TaskSpawn(spawn_error),
            core_error => Self::Core(core_error),
        }
    }
}

impl From<device_envoy_core::led4::Led4BitsToIndexesError> for Error {
    fn from(error: device_envoy_core::led4::Led4BitsToIndexesError) -> Self {
        match error {
            device_envoy_core::led4::Led4BitsToIndexesError::Full => Self::Led4BitsToIndexesFull,
        }
    }
}
