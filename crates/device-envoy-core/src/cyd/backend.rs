//! Backend interfaces used by Device Envoy's platform implementations.
//!
//! Application code does not use this module. Use [`super::Cyd`],
//! [`super::CydDisplay`], and [`super::CydTouch`] instead. This module is public
//! only because the ESP32 and Raspberry Pi implementations live in separate
//! crates.

use super::display::{CydFrame, Orientation};

/// Low-level interface for creating display frames on a new platform.
///
/// Device Envoy's platform crates implement this trait together with
/// [`super::CydDisplay`]. Application code should use [`super::CydDisplay`]
/// instead.
pub trait DisplayBackend {
    /// Error returned when a frame is flushed.
    type Error;

    /// Frame type created by this display implementation.
    type Frame<'a>: CydFrame<Error = Self::Error>
    where
        Self: 'a;

    /// Create a frame covering `rectangle` in logical display coordinates.
    fn create_frame_mut(
        &mut self,
        rectangle: embedded_graphics::primitives::Rectangle,
    ) -> Self::Frame<'_>;
}

/// An uncalibrated touch-controller event read by a platform implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    /// A new touch contact.
    Down {
        /// Raw controller X sample.
        raw_x: u16,
        /// Raw controller Y sample.
        raw_y: u16,
    },
    /// Movement for an existing touch contact.
    Move {
        /// Raw controller X sample.
        raw_x: u16,
        /// Raw controller Y sample.
        raw_y: u16,
    },
    /// The touch contact was released.
    Up,
}

/// The persisted affine mapping from raw controller coordinates to screen coordinates.
pub use super::touch::calibration::CalibrationConfig;

/// Errors returned by the shared calibration driver, retaining device and flash sources.
pub use super::touch::driver::Error;

/// Load or interactively create calibration while constructing a CYD device.
/// Application input handling uses [`super::CydTouch`] instead.
pub use super::touch::driver::ensure_calibration;

/// Raw touch interface implemented by Device Envoy's platform code.
///
/// Device construction converts this interface into a calibrated
/// [`super::CydTouch`] implementation. Application code reads calibrated events
/// through [`super::CydTouch`] instead.
pub trait TouchUncalibrated: Sized {
    /// Error returned when reading raw touch events.
    type Error;

    /// The calibrated touch implementation produced by this backend.
    type Calibrated: super::CydTouch<Error = Self::Error>;

    /// Read the next raw touch event, if any.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Convert this raw-touch implementation into its calibrated touch type.
    fn calibrate(
        self,
        calibration_config: CalibrationConfig,
        orientation: Orientation,
    ) -> Self::Calibrated;
}
