//! Shared touch-calibration math, geometry, and drawing helpers.
//!
//! Calibration lives in the CYD device layer as the single source of truth
//! for affine solve math, corner geometry, drawing helpers, and the sans-io
//! four-tap flow that platform binaries drive with their own touch, logging,
//! persistence, and reset wiring. Start at [`ensure_calibration`].

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::{Rgb888, WebColors},
    primitives::Rectangle,
};
use serde::{Deserialize, Serialize};

use super::RawPoint;
use crate::cyd::display::DrawItem;
use crate::cyd::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub use super::driver::{
    EnsureCalibrationError, EnsureCalibrationErrorKind, EnsureCalibrationOutcome,
    EnsureCalibrationSettings, ensure_calibration, ensure_calibration_with_settings,
};
pub use super::flow::CalibrationFlow;

pub const CALIBRATION_POINT_COUNT: usize = 4;
// Keep the lower crosshair above the bottom 20-pixel calibration message.
pub const CALIBRATION_CROSS_MARGIN: i32 = 40;
pub const CALIBRATION_CROSS_HALF_SIZE: i32 = 18;
pub const CALIBRATION_CENTER_DOT_RADIUS: i32 = 3;
pub const MAX_RESIDUAL_PIXELS: f32 = 12.0;
pub const VERIFY_HIT_RADIUS_PIXELS: f32 = 20.0;

const CALIBRATION_CROSS_COLOR: Rgb888 = Rgb888::CSS_YELLOW;
const CALIBRATION_REJECTED_CROSS_COLOR: Rgb888 = Rgb888::CSS_RED;
const CALIBRATION_DOT_COLOR: Rgb888 = Rgb888::CSS_WHITE;
const AFFINE_DETERMINANT_EPSILON: f32 = 0.000_001;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_SCALE_X: f32 = 1.12;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_SCALE_Y: f32 = 0.93;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_SKEW_X_FROM_Y: f32 = 0.041;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_SKEW_Y_FROM_X: f32 = -0.027;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_OFFSET_X: f32 = 186.0;
#[cfg(any(feature = "wasm", test))]
const DEMO_RAW_OFFSET_Y: f32 = 149.0;

/// Affine mapping from raw controller samples into screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationConfig {
    ax: f32,
    bx: f32,
    cx: f32,
    ay: f32,
    by: f32,
    cy: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationCorner {
    UpperLeft,
    UpperRight,
    LowerRight,
    LowerLeft,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationValidation {
    calibration_config: CalibrationConfig,
    worst_residual_pixels: f32,
}

impl CalibrationValidation {
    #[must_use]
    pub const fn calibration_config(self) -> CalibrationConfig {
        self.calibration_config
    }

    #[cfg(test)]
    #[must_use]
    pub const fn worst_residual_pixels(self) -> f32 {
        self.worst_residual_pixels
    }
}

impl CalibrationConfig {
    #[must_use]
    pub const fn new(ax: f32, bx: f32, cx: f32, ay: f32, by: f32, cy: f32) -> Self {
        Self {
            ax,
            bx,
            cx,
            ay,
            by,
            cy,
        }
    }

    pub fn try_from_four_points(
        points: [RawPoint; CALIBRATION_POINT_COUNT],
    ) -> crate::Result<Self> {
        let screen_targets = [
            calibration_corner_center(CalibrationCorner::UpperLeft),
            calibration_corner_center(CalibrationCorner::UpperRight),
            calibration_corner_center(CalibrationCorner::LowerRight),
            calibration_corner_center(CalibrationCorner::LowerLeft),
        ];

        let (ax, bx, cx) = solve_affine_axis(points, screen_targets, true)?;
        let (ay, by, cy) = solve_affine_axis(points, screen_targets, false)?;

        Ok(Self::new(ax, bx, cx, ay, by, cy))
    }

    #[must_use]
    pub fn from_four_points(points: [RawPoint; CALIBRATION_POINT_COUNT]) -> Self {
        match Self::try_from_four_points(points) {
            Ok(calibration_config) => calibration_config,
            Err(calibration_solve_error) => {
                panic!("invalid touch calibration geometry: {calibration_solve_error:?}")
            }
        }
    }

    #[must_use]
    pub fn map_raw_to_screen(&self, raw_x: u16, raw_y: u16) -> (f32, f32) {
        let raw_x = raw_x as f32;
        let raw_y = raw_y as f32;

        let mapped_x = self.ax * raw_x + self.bx * raw_y + self.cx;
        let mapped_y = self.ay * raw_x + self.by * raw_y + self.cy;

        let mapped_x = mapped_x.clamp(0.0, SCREEN_WIDTH as f32 - 1.0);
        let mapped_y = mapped_y.clamp(0.0, SCREEN_HEIGHT as f32 - 1.0);

        (mapped_x, mapped_y)
    }
}

pub fn validate_calibration_points(
    points: [RawPoint; CALIBRATION_POINT_COUNT],
) -> crate::Result<CalibrationValidation> {
    let calibration_config = CalibrationConfig::try_from_four_points(points)?;
    let worst_residual_pixels = worst_residual_pixels(points, calibration_config);
    if worst_residual_pixels > MAX_RESIDUAL_PIXELS {
        return Err(crate::Error::CalibrationResidualTooLarge {
            worst_residual_pixels,
        });
    }

    Ok(CalibrationValidation {
        calibration_config,
        worst_residual_pixels,
    })
}

#[must_use]
pub const fn calibration_corner_for_index(calibration_index: usize) -> Option<CalibrationCorner> {
    match calibration_index {
        0 => Some(CalibrationCorner::UpperLeft),
        1 => Some(CalibrationCorner::UpperRight),
        2 => Some(CalibrationCorner::LowerRight),
        3 => Some(CalibrationCorner::LowerLeft),
        _ => None,
    }
}

#[must_use]
pub fn calibration_corner_center(calibration_corner: CalibrationCorner) -> Point {
    let width = SCREEN_WIDTH as i32;
    let height = SCREEN_HEIGHT as i32;

    match calibration_corner {
        CalibrationCorner::UpperLeft => {
            Point::new(CALIBRATION_CROSS_MARGIN, CALIBRATION_CROSS_MARGIN)
        }
        CalibrationCorner::UpperRight => Point::new(
            width - 1 - CALIBRATION_CROSS_MARGIN,
            CALIBRATION_CROSS_MARGIN,
        ),
        CalibrationCorner::LowerRight => Point::new(
            width - 1 - CALIBRATION_CROSS_MARGIN,
            height - 1 - CALIBRATION_CROSS_MARGIN,
        ),
        CalibrationCorner::LowerLeft => Point::new(
            CALIBRATION_CROSS_MARGIN,
            height - 1 - CALIBRATION_CROSS_MARGIN,
        ),
    }
}

/// Height, in pixels, of the small text banner `ensure_calibration` draws
/// its instruction/confirmation messages into.
const CALIBRATION_TEXT_HEIGHT: usize = 20;

/// Minimum device static-buffer pixel count required for `ensure_calibration`'s
/// on-screen flow.
///
/// The crosshair/dot geometry streams straight to the panel via
/// [`crate::cyd::CydDisplay::draw_items`] and needs no buffer at all, but the
/// instruction/confirmation text still needs a small buffered frame. Apps
/// that also draw their own status content should size their static buffer
/// to at least `max(CALIBRATION_MIN_PIXEL_COUNT, their_own_needs)` rather
/// than a full-screen buffer — combining a full-screen buffer with, say, a
/// Wi-Fi stack's own heap can overflow memory-constrained boards.
pub const CALIBRATION_MIN_PIXEL_COUNT: usize = SCREEN_WIDTH * CALIBRATION_TEXT_HEIGHT;

/// Rectangle `ensure_calibration` draws its instruction/confirmation text
/// into: a thin banner across the bottom of the screen.
pub(super) const CALIBRATION_TEXT_RECTANGLE: Rectangle = Rectangle::new(
    Point::new(0, SCREEN_HEIGHT as i32 - CALIBRATION_TEXT_HEIGHT as i32),
    Size::new(SCREEN_WIDTH as u32, CALIBRATION_TEXT_HEIGHT as u32),
);

/// Maximum number of [`DrawItem`]s any single calibration redraw produces
/// (a captured-corner dot plus the next corner's crosshair).
pub(super) const CALIBRATION_MAX_DRAW_ITEMS: usize = 4;

/// Draw items for a calibration crosshair with a center dot at `calibration_corner`.
#[must_use]
pub fn calibration_cross_items(calibration_corner: CalibrationCorner) -> [DrawItem; 3] {
    crosshair_items_at(
        calibration_corner_center(calibration_corner),
        CALIBRATION_CROSS_COLOR,
    )
}

/// Like [`calibration_cross_items`], colored to indicate a rejected attempt.
#[must_use]
pub fn calibration_rejected_cross_items(calibration_corner: CalibrationCorner) -> [DrawItem; 3] {
    crosshair_items_at(
        calibration_corner_center(calibration_corner),
        CALIBRATION_REJECTED_CROSS_COLOR,
    )
}

#[must_use]
pub fn calibration_verify_target_center() -> Point {
    Point::new(SCREEN_WIDTH as i32 / 2, SCREEN_HEIGHT as i32 / 2)
}

/// Draw items for the post-calibration verify target at screen center.
#[must_use]
pub fn calibration_verify_target_items() -> [DrawItem; 3] {
    crosshair_items_at(calibration_verify_target_center(), CALIBRATION_CROSS_COLOR)
}

/// Draw item for the small dot acknowledging a captured corner.
#[must_use]
pub fn calibration_ack_dot_item(calibration_corner: CalibrationCorner) -> DrawItem {
    dot_item_at(calibration_corner_center(calibration_corner))
}

fn crosshair_items_at(center: Point, cross_color: Rgb888) -> [DrawItem; 3] {
    let half = CALIBRATION_CROSS_HALF_SIZE as f32;
    let (center_x, center_y) = (center.x as f32, center.y as f32);
    [
        DrawItem::Stroke {
            start: (center_x - half, center_y),
            end: (center_x + half, center_y),
            color: cross_color,
            pixel_width: 4.0,
        },
        DrawItem::Stroke {
            start: (center_x, center_y - half),
            end: (center_x, center_y + half),
            color: cross_color,
            pixel_width: 4.0,
        },
        dot_item_at(center),
    ]
}

fn dot_item_at(center: Point) -> DrawItem {
    DrawItem::Circle {
        center: (center.x as f32, center.y as f32),
        pixel_radius: CALIBRATION_CENTER_DOT_RADIUS as f32,
        color: CALIBRATION_DOT_COLOR,
    }
}

#[cfg(any(feature = "wasm", test))]
#[must_use]
pub fn distort_demo_screen_to_raw(screen_x: f32, screen_y: f32) -> RawPoint {
    let raw_x = DEMO_RAW_SCALE_X * screen_x + DEMO_RAW_SKEW_X_FROM_Y * screen_y + DEMO_RAW_OFFSET_X;
    let raw_y = DEMO_RAW_SKEW_Y_FROM_X * screen_x + DEMO_RAW_SCALE_Y * screen_y + DEMO_RAW_OFFSET_Y;

    assert!(raw_x >= 0.0, "demo raw x must stay non-negative");
    assert!(raw_y >= 0.0, "demo raw y must stay non-negative");
    assert!(raw_x <= u16::MAX as f32, "demo raw x must fit in u16");
    assert!(raw_y <= u16::MAX as f32, "demo raw y must fit in u16");

    RawPoint {
        x: (raw_x + 0.5) as u16,
        y: (raw_y + 0.5) as u16,
    }
}

fn solve_3x3(system_matrix: [[f32; 3]; 3], rhs_vector: [f32; 3]) -> crate::Result<(f32, f32, f32)> {
    let determinant = system_matrix[0][0]
        * (system_matrix[1][1] * system_matrix[2][2] - system_matrix[1][2] * system_matrix[2][1])
        - system_matrix[0][1]
            * (system_matrix[1][0] * system_matrix[2][2]
                - system_matrix[1][2] * system_matrix[2][0])
        + system_matrix[0][2]
            * (system_matrix[1][0] * system_matrix[2][1]
                - system_matrix[1][1] * system_matrix[2][0]);

    if determinant.abs() < AFFINE_DETERMINANT_EPSILON {
        return Err(crate::Error::CalibrationDegenerateGeometry);
    }

    let determinant_ax = rhs_vector[0]
        * (system_matrix[1][1] * system_matrix[2][2] - system_matrix[1][2] * system_matrix[2][1])
        - system_matrix[0][1]
            * (rhs_vector[1] * system_matrix[2][2] - system_matrix[1][2] * rhs_vector[2])
        + system_matrix[0][2]
            * (rhs_vector[1] * system_matrix[2][1] - system_matrix[1][1] * rhs_vector[2]);

    let determinant_bx = system_matrix[0][0]
        * (rhs_vector[1] * system_matrix[2][2] - system_matrix[1][2] * rhs_vector[2])
        - rhs_vector[0]
            * (system_matrix[1][0] * system_matrix[2][2]
                - system_matrix[1][2] * system_matrix[2][0])
        + system_matrix[0][2]
            * (system_matrix[1][0] * rhs_vector[2] - rhs_vector[1] * system_matrix[2][0]);

    let determinant_cx = system_matrix[0][0]
        * (system_matrix[1][1] * rhs_vector[2] - rhs_vector[1] * system_matrix[2][1])
        - system_matrix[0][1]
            * (system_matrix[1][0] * rhs_vector[2] - rhs_vector[1] * system_matrix[2][0])
        + rhs_vector[0]
            * (system_matrix[1][0] * system_matrix[2][1]
                - system_matrix[1][1] * system_matrix[2][0]);

    Ok((
        determinant_ax / determinant,
        determinant_bx / determinant,
        determinant_cx / determinant,
    ))
}

fn solve_affine_axis(
    points: [RawPoint; CALIBRATION_POINT_COUNT],
    screen_targets: [Point; CALIBRATION_POINT_COUNT],
    map_x_axis: bool,
) -> crate::Result<(f32, f32, f32)> {
    let mut sum_xx = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x = 0.0;
    let mut sum_yy = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xo = 0.0;
    let mut sum_yo = 0.0;
    let mut sum_o = 0.0;

    for sample_index in 0..CALIBRATION_POINT_COUNT {
        let raw_x = points[sample_index].x as f32;
        let raw_y = points[sample_index].y as f32;
        let output = if map_x_axis {
            screen_targets[sample_index].x as f32
        } else {
            screen_targets[sample_index].y as f32
        };

        sum_xx += raw_x * raw_x;
        sum_xy += raw_x * raw_y;
        sum_x += raw_x;
        sum_yy += raw_y * raw_y;
        sum_y += raw_y;
        sum_xo += raw_x * output;
        sum_yo += raw_y * output;
        sum_o += output;
    }

    let system_matrix = [
        [sum_xx, sum_xy, sum_x],
        [sum_xy, sum_yy, sum_y],
        [sum_x, sum_y, CALIBRATION_POINT_COUNT as f32],
    ];
    let rhs_vector = [sum_xo, sum_yo, sum_o];
    solve_3x3(system_matrix, rhs_vector)
}

fn worst_residual_pixels(
    points: [RawPoint; CALIBRATION_POINT_COUNT],
    calibration_config: CalibrationConfig,
) -> f32 {
    let calibration_corners = [
        CalibrationCorner::UpperLeft,
        CalibrationCorner::UpperRight,
        CalibrationCorner::LowerRight,
        CalibrationCorner::LowerLeft,
    ];
    let mut worst_residual_pixels = 0.0;

    for (point_index, calibration_corner) in calibration_corners.into_iter().enumerate() {
        let target_point = calibration_corner_center(calibration_corner);
        let raw_point = points[point_index];
        let (mapped_x, mapped_y) = calibration_config.map_raw_to_screen(raw_point.x, raw_point.y);
        let delta_x = mapped_x - target_point.x as f32;
        let delta_y = mapped_y - target_point.y as f32;
        let residual_pixels = micromath::F32Ext::sqrt(delta_x * delta_x + delta_y * delta_y);
        if residual_pixels > worst_residual_pixels {
            worst_residual_pixels = residual_pixels;
        }
    }

    worst_residual_pixels
}

#[cfg(test)]
mod tests {
    use super::{
        CALIBRATION_POINT_COUNT, CalibrationConfig, CalibrationCorner, MAX_RESIDUAL_PIXELS,
        calibration_corner_center, distort_demo_screen_to_raw, validate_calibration_points,
    };

    const MAP_EPSILON: f32 = 0.75;

    #[test]
    fn solve_four_points_recovers_demo_distortion() {
        let calibration_corners = [
            CalibrationCorner::UpperLeft,
            CalibrationCorner::UpperRight,
            CalibrationCorner::LowerRight,
            CalibrationCorner::LowerLeft,
        ];
        let mut raw_points = [distort_demo_screen_to_raw(0.0, 0.0); CALIBRATION_POINT_COUNT];

        for (point_index, calibration_corner) in calibration_corners.into_iter().enumerate() {
            let screen_point = calibration_corner_center(calibration_corner);
            raw_points[point_index] =
                distort_demo_screen_to_raw(screen_point.x as f32, screen_point.y as f32);
        }

        let calibration_config = CalibrationConfig::try_from_four_points(raw_points)
            .expect("demo distortion should solve");

        for (point_index, calibration_corner) in calibration_corners.into_iter().enumerate() {
            let expected = calibration_corner_center(calibration_corner);
            let raw_point = raw_points[point_index];
            let (mapped_x, mapped_y) =
                calibration_config.map_raw_to_screen(raw_point.x, raw_point.y);

            assert!(
                (mapped_x - expected.x as f32).abs() <= MAP_EPSILON,
                "mapped_x={mapped_x} expected_x={}",
                expected.x
            );
            assert!(
                (mapped_y - expected.y as f32).abs() <= MAP_EPSILON,
                "mapped_y={mapped_y} expected_y={}",
                expected.y
            );
        }
    }

    #[test]
    fn contradictory_duplicate_corner_input_is_rejected() {
        let upper_left = distort_demo_screen_to_raw(
            calibration_corner_center(CalibrationCorner::UpperLeft).x as f32,
            calibration_corner_center(CalibrationCorner::UpperLeft).y as f32,
        );
        let upper_right = distort_demo_screen_to_raw(
            calibration_corner_center(CalibrationCorner::UpperRight).x as f32,
            calibration_corner_center(CalibrationCorner::UpperRight).y as f32,
        );
        let lower_left = distort_demo_screen_to_raw(
            calibration_corner_center(CalibrationCorner::LowerLeft).x as f32,
            calibration_corner_center(CalibrationCorner::LowerLeft).y as f32,
        );
        let contradictory_points = [upper_left, upper_right, upper_right, lower_left];

        let error = validate_calibration_points(contradictory_points)
            .expect_err("duplicate-corner input should be rejected");
        assert!(matches!(
            error,
            crate::Error::CalibrationResidualTooLarge { .. }
        ));
    }

    #[test]
    fn clean_input_is_accepted_with_small_residual() {
        let calibration_corners = [
            CalibrationCorner::UpperLeft,
            CalibrationCorner::UpperRight,
            CalibrationCorner::LowerRight,
            CalibrationCorner::LowerLeft,
        ];
        let mut raw_points = [distort_demo_screen_to_raw(0.0, 0.0); CALIBRATION_POINT_COUNT];

        for (point_index, calibration_corner) in calibration_corners.into_iter().enumerate() {
            let screen_point = calibration_corner_center(calibration_corner);
            raw_points[point_index] =
                distort_demo_screen_to_raw(screen_point.x as f32, screen_point.y as f32);
        }

        let validation =
            validate_calibration_points(raw_points).expect("clean demo points should validate");
        assert!(validation.worst_residual_pixels() <= MAX_RESIDUAL_PIXELS);
    }
}
