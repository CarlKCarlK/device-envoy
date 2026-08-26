//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Applications read calibrated screen-space events via [`TouchEvent`]. Platform
//! authors use [`super::backend`] to connect raw controller samples to the
//! private calibration workflow; applications should use [`super::CydTouch`].

pub(crate) mod calibration;
pub(crate) mod driver;
pub(crate) mod flow;

use embedded_graphics::geometry::Point;

/// A touch event in screen coordinates (already calibrated and mapped).
///
/// Read it with [`super::CydTouch::read`] in the canonical [`super::Cyd`]
/// device-loop example. Applications receive screen-space points and do not
/// need to use the backend calibration API.
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
