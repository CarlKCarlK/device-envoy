#[cfg(not(feature = "host"))]
use core::convert::Infallible;

use derive_more::derive::{Display, Error};
use esp_hal_mfrc522::consts::PCDErrorCode;

/// A specialized `Result` where the error is this crate's `Error` type.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Define a unified error type for this crate.
#[expect(missing_docs, reason = "The variants are self-explanatory.")]
#[derive(Debug, Display, Error, derive_more::From)]
pub enum Error {
    // `#[error(not(source))]` below tells `derive_more` that `embassy_executor::SpawnError` does
    // not implement Rust's `core::error::Error` trait.  `SpawnError` should, but Rust's `Error`
    // only recently moved from `std` (which is not available in bare-metal development) to `core`
    // (which is). Perhaps a future update of `embassy_executor::SpawnError` will implement
    // `core::error::Error` which will make this unnecessary.
    #[display("{_0:?}")]
    TaskSpawn(#[error(not(source))] embassy_executor::SpawnError),

    #[display("bits_to_indexes does not have enough preallocated space")]
    BitsToIndexesNotEnoughSpace,

    #[display("BitsToIndexes is full")]
    BitsToIndexesFull,

    #[display("Error setting output state")]
    CannotSetOutputState,

    #[display("Index out of bounds")]
    IndexOutOfBounds,

    #[display("MFRC522 initialization failed: {_0:?}")]
    #[from(ignore)]
    Mfrc522Init(#[error(not(source))] PCDErrorCode),

    #[display("MFRC522 version read failed: {_0:?}")]
    #[from(ignore)]
    Mfrc522Version(#[error(not(source))] PCDErrorCode),

    #[display("Format error")]
    FormatError,

    #[display("Custom WiFi Auto field missing")]
    MissingCustomWifiAutoField,

    #[display("Network Time Protocol (NTP) error: {_0}")]
    Ntp(#[error(not(source))] &'static str),

    #[cfg(feature = "wifi")]
    #[display("DNS error: {_0:?}")]
    Dns(#[error(not(source))] embassy_net::dns::Error),

    #[cfg(not(feature = "host"))]
    #[display("Flash operation failed: {_0:?}")]
    Flash(#[error(not(source))] embassy_rp::flash::Error),

    #[display("Storage is invalid or corrupted")]
    StorageCorrupted,

    #[display("{_0:?}")]
    #[from(ignore)]
    Core(#[error(not(source))] device_envoy_core::Error),

    #[cfg(target_os = "none")]
    #[display("CYD operation failed: {_0:?}")]
    #[from(ignore)]
    // `cyd::Error` is a diagnostic enum but is not itself a `core::error::Error`;
    // keep it as structured context rather than claiming it as a source.
    Cyd(#[error(not(source))] crate::cyd::Error),

    #[cfg(target_os = "none")]
    #[display("CYD touch unavailable")]
    CydTouchUnavailable,
}

#[cfg(target_os = "none")]
impl From<crate::cyd::Error> for Error {
    fn from(error: crate::cyd::Error) -> Self {
        Self::Cyd(error)
    }
}

#[cfg(target_os = "none")]
impl
    From<
        device_envoy_core::cyd::touch::calibration::Error<
            crate::cyd::CydTouchUncalibratedRp,
            Error,
        >,
    > for Error
{
    fn from(
        error: device_envoy_core::cyd::touch::calibration::Error<
            crate::cyd::CydTouchUncalibratedRp,
            Error,
        >,
    ) -> Self {
        match error.kind {
            device_envoy_core::cyd::touch::calibration::ErrorKind::Device(error) => {
                Self::from(error)
            }
            device_envoy_core::cyd::touch::calibration::ErrorKind::Flash(error) => error,
        }
    }
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Self::FormatError
    }
}

#[cfg(not(feature = "host"))]
impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl From<device_envoy_core::Error> for Error {
    fn from(error: device_envoy_core::Error) -> Self {
        #[cfg(feature = "wifi")]
        if let device_envoy_core::Error::TaskSpawn(spawn_error) = error {
            return Self::TaskSpawn(spawn_error);
        }

        Self::Core(error)
    }
}

impl From<device_envoy_core::led4::Led4BitsToIndexesError> for Error {
    fn from(error: device_envoy_core::led4::Led4BitsToIndexesError) -> Self {
        match error {
            device_envoy_core::led4::Led4BitsToIndexesError::Full => Self::BitsToIndexesFull,
        }
    }
}
