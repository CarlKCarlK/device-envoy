//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Apps read calibrated screen-space events via [`TouchEvent`]. Devices also
//! implement [`CydRawTouch`] so the shared touch-calibration flow in
//! [`calibration`] can read raw controller samples.

pub mod calibration;
mod driver;
mod flow;

use embedded_graphics::geometry::Point;

use super::CydFlushError;

/// A touch event in screen coordinates (already calibrated and mapped).
#[derive(Clone, Copy, Debug)]
pub enum TouchEvent {
    Down { point: Point },
    Move { point: Point },
    Up,
}

/// A raw XPT2046 touch sample in controller coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawPoint {
    pub x: u16,
    pub y: u16,
}

/// A raw XPT2046 touch event used by shared calibration flows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    Down { raw_x: u16, raw_y: u16 },
    Move { raw_x: u16, raw_y: u16 },
    Up,
}

/// A CYD raw-touch source implemented by devices so
/// [`calibration::ensure_calibration`] can run.
pub trait CydRawTouch {
    /// Error returned when reading raw touch fails.
    type Error: CydFlushError;

    /// Read the next raw touch event, if any.
    ///
    /// This bypasses any active [`TouchEvent`] calibration mapping and exists
    /// specifically for the shared calibration driver.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;
}
