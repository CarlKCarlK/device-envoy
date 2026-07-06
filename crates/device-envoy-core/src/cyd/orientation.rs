//! Panel orientation for the fixed 320x240 CYD display.

use embedded_graphics::prelude::Size;

use super::{SCREEN_HEIGHT, SCREEN_WIDTH};

/// How the fixed landscape panel is presented.
///
/// Concrete platforms map this to their display driver's rotation; this enum
/// only knows the resulting oriented dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    // todo000 later support a full algebra of rotations and flips like the
    // device-envoy 2D LED panel orientation model.
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
}
