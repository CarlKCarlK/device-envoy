//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Apps read calibrated screen-space events via [`TouchEvent`]. Devices also
//! implement [`CydTouchUncalibrated`] so the shared touch-calibration flow in
//! [`calibration`] can read raw controller samples and transition into a
//! calibrated [`CydTouch`].

pub mod calibration;
pub(crate) mod driver;
pub(crate) mod flow;

use self::calibration::CalibrationConfig;
use embedded_graphics::geometry::Point;

/// A touch event in screen coordinates (already calibrated and mapped).
///
/// See the [CydDisplay trait documentation](super::CydDisplay) for a usage example.
#[derive(Clone, Copy, Debug)]
pub enum TouchEvent {
    Down { point: Point },
    Move { point: Point },
    Up,
}

/// A raw XPT2046 touch sample in controller coordinates.
///
/// See the [touch calibration module documentation](calibration) for usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawPoint {
    pub x: u16,
    pub y: u16,
}

/// A raw XPT2046 touch event used by shared calibration flows.
///
/// See the [touch calibration module documentation](calibration) for usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    Down { raw_x: u16, raw_y: u16 },
    Move { raw_x: u16, raw_y: u16 },
    Up,
}

/// A raw-touch source that can run the shared calibration flow and become calibrated.
pub trait CydTouchUncalibrated: Sized {
    /// Error returned when reading raw touch fails.
    type Error;
    type Calibrated: CydTouch<Error = Self::Error, Uncalibrated = Self>;

    /// Read the next raw touch event, if any.
    ///
    /// This bypasses any active [`TouchEvent`] calibration mapping and exists
    /// specifically for the shared calibration driver.
    /// See the [touch calibration module documentation](calibration) for usage.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Apply `calibration_config`, becoming a calibrated touch source.
    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated;
}

/// A CYD touch source for calibrated, screen-space events that apps read.
///
/// [`CydTouch::read`] returns a [`TouchEvent`] carrying an x-y point in the same
/// screen coordinates as the display, or `None` when there is no touch.
pub trait CydTouch: Sized {
    /// Error returned when reading touch fails.
    type Error;
    type Uncalibrated: CydTouchUncalibrated<Error = Self::Error, Calibrated = Self>;

    /// Read the next calibrated, screen-space touch event, if any.
    ///
    /// Returns `Ok(None)` when there is no pending touch. Errors only on a
    /// hardware/read failure. See the [CydDisplay trait documentation](super::CydDisplay)
    /// for a usage example.
    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error>;

    fn calibration_config(&self) -> CalibrationConfig;

    /// Discard the calibration, becoming an uncalibrated touch source.
    fn decalibrate(self) -> Self::Uncalibrated;
}
