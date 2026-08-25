//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Apps read calibrated screen-space events via [`TouchEvent`]. Devices also
//! Platform backends use [`super::backend`] to connect raw controller samples
//! to the private calibration workflow.

pub(crate) mod calibration;
pub(crate) mod driver;
pub(crate) mod flow;

use embedded_graphics::geometry::Point;

/// A touch event in screen coordinates (already calibrated and mapped).
///
/// See the [`super::Cyd`] trait documentation for a usage example.
#[derive(Clone, Copy, Debug)]
pub enum TouchEvent {
    Down { point: Point },
    Move { point: Point },
    Up,
}

/// A raw XPT2046 touch sample used internally by calibration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawPoint {
    pub x: u16,
    pub y: u16,
}
