//! Backend support for platform-crate authors.
//!
//! This module is public because ESP and RP are separate crates and Rust has no
//! cross-crate `pub(crate)` or friend visibility. Applications should use
//! [`super::Cyd`] and [`super::CydTouch`] instead.
//!
//! ## Platform-author example
//!
//! The following compact example shows both backend seams. Platform authors
//! implement the display and raw-touch traits, pass the persisted calibration
//! through [`ensure_calibration`], and return a calibrated touch type to the
//! application. Application code should use complete-device constructors and
//! calibrated [`super::CydTouch`] reads:
//!
//! ```rust,no_run
//! use core::convert::Infallible;
//! use device_envoy_core::button::Button;
//! use device_envoy_core::cyd::{
//!     CydDisplay,
//!     backend::{CalibrationConfig, DisplayBackend, Error, RawTouchEvent,
//!         TouchUncalibrated, ensure_calibration},
//!     display::Orientation,
//! };
//! use device_envoy_core::flash_block::FlashBlock;
//! use embedded_graphics::{prelude::Point, primitives::Rectangle};
//!
//! fn platform_frame<D: CydDisplay>(display: &mut D) -> D::Frame<'_> {
//!     DisplayBackend::frame_mut_with_tile_top_left(
//!         display,
//!         Rectangle::new(Point::zero(), display.screen_size()),
//!         Point::zero(),
//!     )
//! }
//!
//! fn inspect_raw_touch<T: TouchUncalibrated<Error = Infallible>>(
//!     touch: &mut T,
//! ) -> Result<(), Infallible> {
//!     match touch.read_raw_touch_event()? {
//!         Some(RawTouchEvent::Down { raw_x, raw_y })
//!         | Some(RawTouchEvent::Move { raw_x, raw_y }) => {
//!             assert!(raw_x <= u16::MAX && raw_y <= u16::MAX);
//!         }
//!         Some(RawTouchEvent::Up) | None => {}
//!     }
//!     Ok(())
//! }
//!
//! fn apply_calibration<T: TouchUncalibrated<Error = Infallible>>(
//!     touch: T,
//!     config: CalibrationConfig,
//! ) -> T::Calibrated {
//!     let landscape_point = config.map_raw_to_screen(100, 200);
//!     assert!((0.0..320.0).contains(&landscape_point.0));
//!     assert!((0.0..240.0).contains(&landscape_point.1));
//!     touch.calibrate(config, Orientation::Landscape)
//! }
//!
//! async fn calibrate<D, T, F, R>(
//!     display: &mut D,
//!     touch: T,
//!     flash: &mut F,
//!     button: &mut R,
//! ) -> Result<T::Calibrated, Error<D::Error, F::Error>>
//! where
//!     D: CydDisplay,
//!     T: TouchUncalibrated<Error = D::Error>,
//!     F: FlashBlock,
//!     R: Button,
//! {
//!     ensure_calibration(
//!         display,
//!         touch,
//!         flash,
//!         button,
//!         Some("Touch calibrated"),
//!         Orientation::Landscape,
//!     )
//!     .await
//! }
//!
//! fn classify(error: Error<Infallible, Infallible>) {
//!     match error {
//!         Error::Device(_) | Error::Flash(_) => {}
//!     }
//! }
//! ```

use super::display::{CydFrame, Orientation};

/// The platform-only display construction seam used by ESP and RP backends.
///
/// This is public because those implementations live in separate crates and
/// cannot implement a private core-only hook. The platform-author example is
/// on [`crate::cyd::backend`]. Applications should use
/// [`super::CydDisplay::frame_mut`], [`super::CydDisplay::full_frame_mut`], or
/// [`super::CydDisplay::for_each_tile`] instead.
pub trait DisplayBackend {
    /// Error returned when a frame is flushed.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    type Error;

    /// Frame type produced by this backend.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    type Frame<'a>: CydFrame<Error = Self::Error>
    where
        Self: 'a;

    /// Construct a frame whose drawing coordinates are translated from screen
    /// coordinates by `tile_top_left`.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    fn frame_mut_with_tile_top_left(
        &mut self,
        rectangle: embedded_graphics::primitives::Rectangle,
        tile_top_left: embedded_graphics::prelude::Point,
    ) -> Self::Frame<'_>;
}

/// A raw XPT2046 touch event used by platform calibration backends.
/// See the platform-author example on [`crate::cyd::backend`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawTouchEvent {
    /// A new touch contact.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    Down {
        /// Raw controller X sample.
        /// See the platform-author example on [`crate::cyd::backend`].
        raw_x: u16,
        /// Raw controller Y sample.
        /// See the platform-author example on [`crate::cyd::backend`].
        raw_y: u16,
    },
    /// Movement for an existing touch contact.
    /// See the platform-author example on [`crate::cyd::backend`].
    Move {
        /// Raw controller X sample.
        /// See the platform-author example on [`crate::cyd::backend`].
        raw_x: u16,
        /// Raw controller Y sample.
        /// See the platform-author example on [`crate::cyd::backend`].
        raw_y: u16,
    },
    /// The touch contact was released.
    /// See the platform-author example on [`crate::cyd::backend`].
    Up,
}

/// The persisted affine mapping from raw controller coordinates to screen coordinates.
///
/// See the platform-author example on [`crate::cyd::backend`].
pub use super::touch::calibration::CalibrationConfig;

/// Errors returned by the shared calibration driver, retaining device and flash sources.
///
/// See the platform-author example on [`crate::cyd::backend`].
pub use super::touch::driver::Error;

/// Load or interactively create calibration for a platform touch backend.
///
/// See the platform-author example on [`crate::cyd::backend`]. Applications should
/// use calibrated [`super::CydTouch`] reads instead.
pub use super::touch::driver::ensure_calibration;

/// A platform touch implementation that can be calibrated by the CYD
/// constructor workflow. This backend seam is public only because ESP and RP
/// are separate crates; applications should use [`super::CydTouch`]. See the
/// platform-author example on [`crate::cyd::backend`].
pub trait TouchUncalibrated: Sized {
    /// Error returned when reading raw touch events.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
    type Error;

    /// The calibrated touch implementation produced by this backend.
    ///
    /// See the platform-author example on [`crate::cyd::backend`].
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
