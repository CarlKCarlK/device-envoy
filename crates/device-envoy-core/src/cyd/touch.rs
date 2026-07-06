//! Touch-side support types for the CYD's `cyd` device abstraction.
//!
//! Apps read calibrated screen-space events via [`TouchEvent`]. Devices also
//! implement [`CydRawTouch`] so the shared touch-calibration flow in
//! [`calibration`] can read raw controller samples.

pub mod calibration;
pub mod driver;
pub mod flow;

use embedded_graphics::geometry::Point;

use super::CydFlushError;

pub use calibration::{
    CALIBRATION_CENTER_DOT_RADIUS, CALIBRATION_CROSS_HALF_SIZE, CALIBRATION_CROSS_MARGIN,
    CALIBRATION_POINT_COUNT, CalibrationConfig, CalibrationCorner, CalibrationFlow,
    CalibrationValidation, MAX_RESIDUAL_PIXELS, VERIFY_HIT_RADIUS_PIXELS,
    calibration_corner_center, calibration_corner_for_index, calibration_verify_target_center,
    distort_demo_screen_to_raw, draw_calibration_ack_dot, draw_calibration_cross,
    draw_calibration_instruction, draw_calibration_rejected_cross,
    draw_calibration_verify_target, validate_calibration_points,
};
pub use driver::{
    EnsureCalibrationError, EnsureCalibrationOutcome, EnsureCalibrationSettings,
    ensure_calibration, ensure_calibration_with_settings,
};

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

/// A CYD raw-touch source implemented by devices so [`ensure_calibration`] can run.
pub trait CydRawTouch {
    /// Error returned when reading raw touch fails.
    type Error: CydFlushError;

    /// Read the next raw touch event, if any.
    ///
    /// This bypasses any active [`TouchEvent`] calibration mapping and exists
    /// specifically for the shared calibration driver.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;
}
