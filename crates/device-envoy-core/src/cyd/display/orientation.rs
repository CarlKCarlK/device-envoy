//! Panel orientation for the fixed 320x240 CYD display.

use embedded_graphics::geometry::Point;
use embedded_graphics::prelude::Size;

use super::super::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// How the fixed landscape panel is presented.
///
/// Concrete platforms map this to their display driver's rotation; this enum
/// only knows the resulting oriented dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Orientation {
    Landscape,
    Portrait,
    LandscapeInverted,
    PortraitInverted,
}

impl Orientation {
    #[must_use]
    pub const fn width(self) -> u32 {
        match self {
            Self::Landscape | Self::LandscapeInverted => SCREEN_WIDTH as u32,
            Self::Portrait | Self::PortraitInverted => SCREEN_HEIGHT as u32,
        }
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        match self {
            Self::Landscape | Self::LandscapeInverted => SCREEN_HEIGHT as u32,
            Self::Portrait | Self::PortraitInverted => SCREEN_WIDTH as u32,
        }
    }

    #[must_use]
    pub const fn size(self) -> Size {
        Size::new(self.width(), self.height())
    }

    #[must_use]
    pub const fn pixels(self) -> usize {
        self.width() as usize * self.height() as usize
    }

    /// Return the next orientation in the four-state display test cycle.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Landscape => Self::Portrait,
            Self::Portrait => Self::LandscapeInverted,
            Self::LandscapeInverted => Self::PortraitInverted,
            Self::PortraitInverted => Self::Landscape,
        }
    }

    /// Map a calibrated landscape point into this orientation's screen space.
    ///
    /// Touch calibration always describes the fixed landscape panel. The DNS
    /// application uses this conversion when the display is presented in one
    /// of the three alternate orientations.
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
