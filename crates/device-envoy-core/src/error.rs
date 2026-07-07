//! Shared error and result types for `device-envoy-core`.

use core::convert::Infallible;

/// A specialized `Result` where the error is this crate's [`Error`] type.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Extension for unwrapping a `Result` whose error type is [`Infallible`].
pub trait UnwrapInfallible {
    /// Success value produced by the result.
    type Output;

    /// Unwrap a `Result<T, Infallible>` without a possible panic path.
    fn unwrap_infallible(self) -> Self::Output;
}

impl<T> UnwrapInfallible for core::result::Result<T, Infallible> {
    type Output = T;

    fn unwrap_infallible(self) -> T {
        match self {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }
}

/// Unified error type for `device-envoy-core`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Spawning an Embassy task failed.
    #[cfg(feature = "wifi")]
    TaskSpawn(embassy_executor::SpawnError),

    /// A pixel copy's source slice length did not match the destination frame length.
    CopySize {
        /// Length of the source pixel slice.
        src_len: usize,
        /// Length of the destination frame.
        frame_len: usize,
    },

    /// An I2C write to the character LCD's expander failed for the given address.
    LcdI2cWrite {
        /// The 7-bit I2C address that failed to write.
        address: u8,
    },

    /// Attempted to set the character LCD cursor to an out-of-range row.
    LcdRowOutOfBounds {
        /// The out-of-range row index.
        row: usize,
    },

    /// Touch calibration input geometry was degenerate and could not be solved.
    CalibrationDegenerateGeometry,

    /// Touch calibration solved, but the residual error was too large to accept.
    CalibrationResidualTooLarge {
        /// The worst observed residual, in pixels.
        worst_residual_pixels: f32,
    },

    /// Captive-portal data or rendering format was invalid.
    WifiAutoFormat,

    /// Stored Wi-Fi auto state is invalid for expected runtime flow.
    WifiAutoStorageCorrupted,

    /// A required custom field is missing from the Wi-Fi auto setup.
    WifiAutoMissingCustomField,
}

#[cfg(feature = "wifi")]
impl From<embassy_executor::SpawnError> for Error {
    fn from(err: embassy_executor::SpawnError) -> Self {
        Self::TaskSpawn(err)
    }
}
