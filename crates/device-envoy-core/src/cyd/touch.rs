//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Apps read calibrated screen-space events via [`TouchEvent`]. Devices also
//! implement [`super::CydTouchUncalibrated`] so the shared touch-calibration
//! flow in [`calibration`] can read raw controller samples and transition
//! into a calibrated [`super::CydTouch`].

pub mod calibration;
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
