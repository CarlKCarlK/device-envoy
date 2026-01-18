#![cfg(feature = "host")]
//cmk000000000000000 we can't have this file!

use core::ops::{Deref, DerefMut};
use embedded_graphics::prelude::RgbColor;

/// Predefined RGB color constants from the `smart_leds` crate.
#[doc(inline)]
pub use smart_leds::colors;

/// 8-bit-per-channel RGB color from `embedded_graphics`.
#[doc(inline)]
pub use embedded_graphics::pixelcolor::Rgb888;

/// RGB color type used by LED strip frames.
pub use smart_leds::RGB8;

/// Convert [`RGB8`](https://docs.rs/smart-leds/latest/smart_leds/type.RGB8.html) ([`smart_leds`](https://docs.rs/smart-leds/latest/smart_leds/index.html)) to
/// [`Rgb888`](https://docs.rs/embedded-graphics/latest/embedded_graphics/pixelcolor/struct.Rgb888.html) (embedded_graphics).
#[must_use]
pub const fn rgb8_to_rgb888(color: RGB8) -> Rgb888 {
    Rgb888::new(color.r, color.g, color.b)
}

/// Convert [`Rgb888`](https://docs.rs/embedded-graphics/latest/embedded_graphics/pixelcolor/struct.Rgb888.html) ([embedded_graphics](https://docs.rs/embedded-graphics/latest/embedded_graphics/)) to
/// [`RGB8`](https://docs.rs/smart-leds/latest/smart_leds/type.RGB8.html) ([smart_leds](https://docs.rs/smart-leds/latest/smart_leds/index.html)).
#[must_use]
pub fn rgb888_to_rgb8(color: Rgb888) -> RGB8 {
    RGB8::new(color.r(), color.g(), color.b())
}

/// Fixed-size 1D LED strip frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame1d<const N: usize>(pub [RGB8; N]);

impl<const N: usize> Frame1d<N> {
    /// Number of LEDs in this frame.
    pub const LEN: usize = N;

    /// Create a new blank (all black) frame.
    #[must_use]
    pub const fn new() -> Self {
        Self([RGB8::new(0, 0, 0); N])
    }

    /// Create a frame filled with a single color.
    #[must_use]
    pub const fn filled(color: RGB8) -> Self {
        Self([color; N])
    }
}

impl<const N: usize> Deref for Frame1d<N> {
    type Target = [RGB8; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for Frame1d<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> From<[RGB8; N]> for Frame1d<N> {
    fn from(array: [RGB8; N]) -> Self {
        Self(array)
    }
}

impl<const N: usize> From<Frame1d<N>> for [RGB8; N] {
    fn from(frame: Frame1d<N>) -> Self {
        frame.0
    }
}

impl<const N: usize> Default for Frame1d<N> {
    fn default() -> Self {
        Self::new()
    }
}
