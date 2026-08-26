//! Backend support for platform-crate authors.
//!
//! This module is public because ESP and RP are separate crates and Rust has no
//! cross-crate `pub(crate)` or friend visibility. Applications should use
//! [`super::Cyd`] and [`super::CydTouch`] instead.
//!
//! The following compact example shows the seam a platform crate implements;
//! application code should use the complete-device constructors and calibrated
//! [`super::CydTouch`] reads:
//!
//! ```rust,no_run
//! use device_envoy_core::cyd::backend::{
//!     CalibrationConfig, Error, RawTouchEvent, TouchUncalibrated, ensure_calibration,
//! };
//! use device_envoy_core::{
//!     button::Button,
//!     cyd::CydDisplay,
//!     flash_block::FlashBlock,
//! };
//!
//! fn platform_backend<D, T, F, R>()
//! where
//!     D: CydDisplay,
//!     T: TouchUncalibrated<Error = D::Error>,
//!     F: FlashBlock,
//!     R: Button,
//! {
//!     let _raw_event = RawTouchEvent::Up;
//!     let _saved_config: Option<CalibrationConfig> = None;
//!     let _backend_error: Option<Error<D::Error, F::Error>> = None;
//!     let _calibration_driver = ensure_calibration::<D, T, F, R>;
//! }
//! ```

use super::display::{CydFrame, Orientation};

/// The platform-only display construction seam used by ESP and RP backends.
///
/// This is public because those implementations live in separate crates and
/// cannot implement a private core-only hook. The platform-author example is
/// [`crate::cyd::backend`]. Applications should use
/// [`super::CydDisplay::frame_mut`], [`super::CydDisplay::full_frame_mut`], or
/// [`super::CydDisplay::for_each_tile`] instead.
pub trait DisplayBackend {
    /// Error returned when a frame is flushed.
    type Error;

    /// Frame type produced by this backend.
    type Frame<'a>: CydFrame<Error = Self::Error>
    where
        Self: 'a;

    /// Construct a frame whose drawing coordinates are translated from screen
    /// coordinates by `tile_top_left`.
    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: embedded_graphics::primitives::Rectangle,
        tile_top_left: embedded_graphics::prelude::Point,
    ) -> Self::Frame<'_>;
}

/// A raw XPT2046 touch event used by platform calibration backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    /// A new touch contact.
    ///
    /// See the [backend example](self) for the platform-author seam.
    Down { raw_x: u16, raw_y: u16 },
    /// Movement for an existing touch contact.
    Move { raw_x: u16, raw_y: u16 },
    /// The touch contact was released.
    Up,
}

/// The persisted affine mapping from raw controller coordinates to screen coordinates.
///
/// See the [`ensure_calibration`] platform-author example.
pub use super::touch::calibration::CalibrationConfig;

/// Errors returned by the shared calibration driver, retaining device and flash sources.
///
/// See the [`ensure_calibration`] platform-author example.
pub use super::touch::driver::Error;

/// Load or interactively create calibration for a platform touch backend.
///
/// See the platform-author example on [`crate::cyd::backend`]. Applications should
/// use calibrated [`super::CydTouch`] reads instead.
pub use super::touch::driver::ensure_calibration;

/// A platform touch implementation that can be calibrated by the CYD
/// constructor workflow. This backend seam is public only because ESP and RP
/// are separate crates; applications should use [`super::CydTouch`].
pub trait TouchUncalibrated: Sized {
    /// See the platform-author example on [`crate::cyd::backend`].
    /// Error returned when reading raw touch events.
    type Error;
    /// See the platform-author example on [`crate::cyd::backend`].
    /// The calibrated touch implementation produced by this backend.
    type Calibrated: super::CydTouch<Error = Self::Error>;

    /// Read the next raw touch event, if any.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Apply a saved or newly solved calibration.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    fn calibrate(
        self,
        calibration_config: CalibrationConfig,
        orientation: Orientation,
    ) -> Self::Calibrated;
}
