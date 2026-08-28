//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Applications read calibrated, oriented events via [`TouchEvent`]. Device
//! Envoy's platform implementations connect raw controller samples to the
//! private calibration workflow. Applications use
//! [`CydTouch`](super::CydTouch); see its
//! [example](super::CydTouch::try_read).

pub(crate) mod calibration;
pub(crate) mod driver;
pub(crate) mod flow;

use embedded_graphics::geometry::Point;

/// A touch event in logical display coordinates (already calibrated and oriented).
///
/// Read it with [CydTouch::try_read](super::CydTouch::try_read) in the
/// [focused example](super::CydTouch::try_read), or see the larger
/// [application example](../index.html#application-example) for a complete
/// read-and-draw flow. Applications receive coordinates bounded by
/// [`CydDisplay::screen_size()`](super::CydDisplay::screen_size) and must not
/// apply another orientation mapping.
/// Calibration remains defined against the fixed `320×240` landscape panel
/// internally; the platform touch implementation maps the result into the
/// runtime display orientation before returning it.
#[derive(Clone, Copy, Debug)]
pub enum TouchEvent {
    /// The touch contact began at `point` in logical display coordinates.
    /// See the compiled [`CydTouch::try_read`](super::CydTouch::try_read) example.
    Down {
        /// Calibrated point bounded by the display's logical size.
        /// See the compiled [`CydTouch::try_read`](super::CydTouch::try_read) example.
        point: Point,
    },
    /// The active touch contact moved to `point` in logical display coordinates.
    /// See the compiled [`CydTouch::try_read`](super::CydTouch::try_read) example.
    Move {
        /// Calibrated point bounded by the display's logical size.
        /// See the compiled [`CydTouch::try_read`](super::CydTouch::try_read) example.
        point: Point,
    },
    /// The touch contact ended; no point is carried.
    /// See the compiled [`CydTouch::try_read`](super::CydTouch::try_read) example.
    Up,
}

/// A raw XPT2046 touch sample used internally by calibration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawPoint {
    pub x: u16,
    pub y: u16,
}
