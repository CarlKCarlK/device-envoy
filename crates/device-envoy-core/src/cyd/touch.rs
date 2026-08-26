//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Applications read calibrated, oriented events via [`TouchEvent`]. Platform
//! authors connect raw controller samples to the private calibration workflow
//! through the backend seam; applications should use [CydTouch](super::CydTouch). See
//! the [canonical calibrated-read example](super::CydTouch::read).

pub(crate) mod calibration;
pub(crate) mod driver;
pub(crate) mod flow;

use embedded_graphics::geometry::Point;

/// A touch event in logical display coordinates (already calibrated and oriented).
///
/// Read it with [CydTouch::read](super::CydTouch::read) in the [canonical calibrated-read
/// example](super::CydTouch::read). Applications receive coordinates bounded by
/// `CydDisplay::screen_size()` and must not apply another orientation mapping.
/// Calibration remains defined against the fixed `320×240` landscape panel
/// internally; the platform touch implementation maps the result into the
/// runtime display orientation before returning it.
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
