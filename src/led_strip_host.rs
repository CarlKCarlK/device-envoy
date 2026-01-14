#![cfg(feature = "host")]

use core::ops::{Deref, DerefMut};

/// Predefined RGB color constants from the `smart_leds` crate.
#[doc(inline)]
pub use smart_leds::colors;

/// RGB color type used by LED strip frames.
pub type Rgb = smart_leds::RGB8;

/// Fixed-size 1D LED strip frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame1d<const N: usize>(pub [Rgb; N]);

impl<const N: usize> Frame1d<N> {
    /// Number of LEDs in this frame.
    pub const LEN: usize = N;

    /// Create a new blank (all black) frame.
    #[must_use]
    pub const fn new() -> Self {
        Self([Rgb::new(0, 0, 0); N])
    }

    /// Create a frame filled with a single color.
    #[must_use]
    pub const fn filled(color: Rgb) -> Self {
        Self([color; N])
    }
}

impl<const N: usize> Deref for Frame1d<N> {
    type Target = [Rgb; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for Frame1d<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> From<[Rgb; N]> for Frame1d<N> {
    fn from(array: [Rgb; N]) -> Self {
        Self(array)
    }
}

impl<const N: usize> From<Frame1d<N>> for [Rgb; N] {
    fn from(frame: Frame1d<N>) -> Self {
        frame.0
    }
}

impl<const N: usize> Default for Frame1d<N> {
    fn default() -> Self {
        Self::new()
    }
}
