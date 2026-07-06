//! Shared touch-calibration math, geometry, and drawing helpers.
//!
//! Calibration lives in the CYD device layer as the single source of truth
//! for affine solve math, corner geometry, drawing helpers, and the sans-io
//! four-tap flow that platform binaries drive with their own touch, logging,
//! persistence, and reset wiring. Start at [`super::ensure_calibration`].

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::{Rgb565, Rgb888, WebColors},
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle},
    text::{Baseline, Text},
};
use serde::{Deserialize, Serialize};

use super::{CalibrationFlow, RawPoint, RawTouchEvent};
use crate::cyd::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub const CALIBRATION_POINT_COUNT: usize = 4;
pub const CALIBRATION_CROSS_MARGIN: i32 = 28;
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

/// Draw a calibration crosshair with a center dot onto `target`.
pub fn draw_calibration_cross<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
    calibration_corner: CalibrationCorner,
) -> Result<(), E> {
    draw_crosshair_at(
        target,
        calibration_corner_center(calibration_corner),
        Rgb565::from(CALIBRATION_CROSS_COLOR),
        Rgb565::from(CALIBRATION_DOT_COLOR),
    )
}

pub fn draw_calibration_rejected_cross<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
    calibration_corner: CalibrationCorner,
) -> Result<(), E> {
    draw_crosshair_at(
        target,
        calibration_corner_center(calibration_corner),
        Rgb565::from(CALIBRATION_REJECTED_CROSS_COLOR),
        Rgb565::from(CALIBRATION_DOT_COLOR),
    )
}

#[must_use]
pub fn calibration_verify_target_center() -> Point {
    Point::new(SCREEN_WIDTH as i32 / 2, SCREEN_HEIGHT as i32 / 2)
}

pub fn draw_calibration_verify_target<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
) -> Result<(), E> {
    draw_crosshair_at(
        target,
        calibration_verify_target_center(),
        Rgb565::from(CALIBRATION_CROSS_COLOR),
        Rgb565::from(CALIBRATION_DOT_COLOR),
    )
}

pub fn draw_calibration_instruction<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
    message: &str,
) -> Result<(), E> {
    Text::with_baseline(
        message,
        Point::new(12, SCREEN_HEIGHT as i32 - 14),
        MonoTextStyle::new(&FONT_6X10, Rgb565::from(CALIBRATION_DOT_COLOR)),
        Baseline::Top,
    )
    .draw(target)?;
    Ok(())
}

pub fn draw_calibration_ack_dot<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
    calibration_corner: CalibrationCorner,
) -> Result<(), E> {
    Circle::new(
        calibration_corner_center(calibration_corner)
            - Point::new(CALIBRATION_CENTER_DOT_RADIUS, CALIBRATION_CENTER_DOT_RADIUS),
        (CALIBRATION_CENTER_DOT_RADIUS * 2) as u32,
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::from(
        CALIBRATION_DOT_COLOR,
    )))
    .draw(target)?;
    Ok(())
}

fn draw_crosshair_at<E>(
    target: &mut impl DrawTarget<Color = Rgb565, Error = E>,
    center: Point,
    cross_color: Rgb565,
    dot_color: Rgb565,
) -> Result<(), E> {
    let left = Point::new(center.x - CALIBRATION_CROSS_HALF_SIZE, center.y);
    let right = Point::new(center.x + CALIBRATION_CROSS_HALF_SIZE, center.y);
    let top = Point::new(center.x, center.y - CALIBRATION_CROSS_HALF_SIZE);
    let bottom = Point::new(center.x, center.y + CALIBRATION_CROSS_HALF_SIZE);

    Line::new(left, right)
        .into_styled(PrimitiveStyle::with_stroke(cross_color, 4))
        .draw(target)?;
    Line::new(top, bottom)
        .into_styled(PrimitiveStyle::with_stroke(cross_color, 4))
        .draw(target)?;

    Circle::new(
        Point::new(
            center.x - CALIBRATION_CENTER_DOT_RADIUS,
            center.y - CALIBRATION_CENTER_DOT_RADIUS,
        ),
        (CALIBRATION_CENTER_DOT_RADIUS * 2 + 1) as u32,
    )
    .into_styled(PrimitiveStyle::with_fill(dot_color))
    .draw(target)?;

    Ok(())
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
