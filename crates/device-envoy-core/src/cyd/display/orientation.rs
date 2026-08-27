//! Panel orientation for the fixed 320×240 CYD display.

use embedded_graphics::geometry::Point;
use embedded_graphics::prelude::Size;

use super::super::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// How the fixed landscape panel is presented.
///
/// `Landscape`, `Portrait`, `LandscapeInverted`, and `PortraitInverted` name
/// the four display layouts. A complete device applies the selected layout to
/// calibrated touch samples before returning them. Applications consume those
/// logical points directly; they do not map a [`TouchEvent`](crate::cyd::touch::TouchEvent)
/// again.
///
/// Concrete platforms map this to their display driver's rotation; this enum
/// only knows the resulting oriented dimensions.
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::Orientation;
/// use embedded_graphics::prelude::{Point, Size};
///
/// for orientation in [
///     Orientation::Landscape,
///     Orientation::Portrait,
///     Orientation::LandscapeInverted,
///     Orientation::PortraitInverted,
/// ] {
///     assert!(orientation.width() > 0 && orientation.height() > 0);
/// }
/// let landscape = Orientation::Landscape;
/// assert_eq!(landscape.size(), Size::new(320, 240));
/// assert_eq!(landscape.pixels(), 320 * 240);
/// assert_eq!(landscape.next(), Orientation::Portrait);
/// assert_eq!(landscape.map_landscape_point(Point::new(10, 20)), Point::new(10, 20));
/// assert_eq!(Orientation::Portrait.map_landscape_point(Point::new(10, 20)), Point::new(20, 309));
/// assert_eq!(Orientation::LandscapeInverted.map_landscape_point(Point::new(10, 20)), Point::new(309, 219));
/// assert_eq!(Orientation::PortraitInverted.map_landscape_point(Point::new(10, 20)), Point::new(219, 10));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Orientation {
    /// Native 320×240 landscape presentation.
    /// See the compiled canonical [`Orientation`] example.
    Landscape,
    /// 240×320 portrait presentation, rotated clockwise.
    /// See the compiled canonical [`Orientation`] example.
    Portrait,
    /// Native landscape presentation rotated 180°.
    /// See the compiled canonical [`Orientation`] example.
    LandscapeInverted,
    /// 240×320 portrait presentation, rotated counterclockwise.
    /// See the compiled canonical [`Orientation`] example.
    PortraitInverted,
}

impl Orientation {
    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn width(self) -> u32 {
        match self {
            Self::Landscape | Self::LandscapeInverted => SCREEN_WIDTH as u32,
            Self::Portrait | Self::PortraitInverted => SCREEN_HEIGHT as u32,
        }
    }

    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn height(self) -> u32 {
        match self {
            Self::Landscape | Self::LandscapeInverted => SCREEN_HEIGHT as u32,
            Self::Portrait | Self::PortraitInverted => SCREEN_WIDTH as u32,
        }
    }

    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn size(self) -> Size {
        Size::new(self.width(), self.height())
    }

    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn pixels(self) -> usize {
        self.width() as usize * self.height() as usize
    }

    /// Return the next orientation in the four-state display test cycle.
    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Landscape => Self::Portrait,
            Self::Portrait => Self::LandscapeInverted,
            Self::LandscapeInverted => Self::PortraitInverted,
            Self::PortraitInverted => Self::Landscape,
        }
    }

    /// Map a fixed-landscape panel point into logical display coordinates.
    ///
    /// Touch calibration always describes the fixed 320×240 landscape panel.
    /// Use this for panel/calibration coordinates or fixed-landscape assets.
    /// Do not use it on an application-facing [`TouchEvent`](crate::cyd::touch::TouchEvent):
    /// [`CydTouch::read`](crate::cyd::CydTouch::read) has already applied this
    /// mapping exactly once.
    /// See the [canonical `Orientation` example](Orientation).
    #[must_use]
    pub const fn map_landscape_point(self, point: Point) -> Point {
        let landscape_width = SCREEN_WIDTH as i32;
        let landscape_height = SCREEN_HEIGHT as i32;
        match self {
            Self::Landscape => point,
            Self::Portrait => Point::new(point.y, landscape_width - 1 - point.x),
            Self::LandscapeInverted => Point::new(
                landscape_width - 1 - point.x,
                landscape_height - 1 - point.y,
            ),
            Self::PortraitInverted => Point::new(landscape_height - 1 - point.y, point.x),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Orientation;
    use embedded_graphics::geometry::Point;

    const ORIENTATIONS: [Orientation; 4] = [
        Orientation::Landscape,
        Orientation::Portrait,
        Orientation::LandscapeInverted,
        Orientation::PortraitInverted,
    ];

    #[test]
    fn landscape_points_map_inside_each_oriented_screen() {
        for orientation in ORIENTATIONS {
            for position_y in 0..240 {
                for position_x in 0..320 {
                    let mapped_point =
                        orientation.map_landscape_point(Point::new(position_x, position_y));
                    assert!(mapped_point.x >= 0);
                    assert!(mapped_point.y >= 0);
                    assert!(mapped_point.x < orientation.width() as i32);
                    assert!(mapped_point.y < orientation.height() as i32);
                }
            }
        }
    }

    #[test]
    fn orientation_mapping_round_trips_all_landscape_pixels() {
        for orientation in ORIENTATIONS {
            for position_y in 0..240 {
                for position_x in 0..320 {
                    let landscape_point = Point::new(position_x, position_y);
                    let logical_point = orientation.map_landscape_point(landscape_point);
                    assert_eq!(
                        inverse_map_logical_point(orientation, logical_point),
                        landscape_point
                    );
                }
            }
        }
    }

    #[test]
    fn orientation_cycle_visits_each_state_once() {
        for orientation in ORIENTATIONS {
            let mut next_orientation = orientation;
            for expected_orientation in [
                orientation.next(),
                orientation.next().next(),
                orientation.next().next().next(),
                orientation,
            ] {
                next_orientation = next_orientation.next();
                assert_eq!(next_orientation, expected_orientation);
            }
        }
    }

    const fn inverse_map_logical_point(orientation: Orientation, point: Point) -> Point {
        match orientation {
            Orientation::Landscape => point,
            Orientation::Portrait => Point::new(319 - point.y, point.x),
            Orientation::LandscapeInverted => Point::new(319 - point.x, 239 - point.y),
            Orientation::PortraitInverted => Point::new(point.y, 239 - point.x),
        }
    }
}
