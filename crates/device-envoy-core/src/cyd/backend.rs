//! Backend support for platform-crate authors.
//!
//! This module is public because ESP and RP are separate crates and Rust has no
//! cross-crate `pub(crate)` or friend visibility. Applications should use
//! [`super::Cyd`] and [`super::CydTouch`] instead.

/// A raw XPT2046 touch event used by platform calibration backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    /// A new touch contact.
    Down { raw_x: u16, raw_y: u16 },
    /// Movement for an existing touch contact.
    Move { raw_x: u16, raw_y: u16 },
    /// The touch contact was released.
    Up,
}

pub use super::touch::calibration::CalibrationConfig;
pub use super::touch::driver::{Error, ensure_calibration};

/// A platform touch implementation that can be calibrated by the CYD
/// constructor workflow. This backend seam is public only because ESP and RP
/// are separate crates; applications should use [`super::CydTouch`].
pub trait TouchUncalibrated: Sized {
    /// Error returned when reading raw touch events.
    type Error;
    /// The calibrated touch implementation produced by this backend.
    type Calibrated: super::CydTouch<Error = Self::Error>;

    /// Read the next raw touch event, if any.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Apply a saved or newly solved calibration.
    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated;
}
