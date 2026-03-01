#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_simple",
        "docs/assets/led_strip_simple.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gpio0",
        "docs/assets/led_strip_gpio0.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gogo",
        "docs/assets/led2d2.png"
    ),
    doc = ::embed_doc_image::embed_image!(
        "led_strip_animated",
        "docs/assets/led_strip_animated.png"
    )
)]
//! A device abstraction for 1-dimensional NeoPixel-style (WS2812) LED strips. For 2-dimensional
//! panels, see the [`led2d`](mod@crate::led2d) module.
//!
//! This page provides the primary documentation and examples for programming LED strips.
//! The device abstraction supports pixel patterns and animation on the LED strip.
//!
//! **After reading the examples below, see also:**
//!
//! - [`led_strip!`](macro@crate::led_strip) — Macro to generate an LED strip struct type (includes syntax details). See [`LedStripGenerated`](led_strip_generated::LedStripGenerated) for a sample of a generated type.
//! - [`LedStripGenerated`](led_strip_generated::LedStripGenerated) — Sample struct type showing all methods and associated constants.
//! - [`Frame1d`] — 1D pixel array used to describe LED strip patterns.
//! - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other strips or panels (includes examples).
//!
//! # Example: Write a Single 1-Dimensional Frame
//!
//! In this example, we set every other LED to blue and gray. Here, the generated struct type is
//! named `LedStripSimple`.
//!
//! ![LED strip preview][led_strip_simple]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy::{Result, led_strip::{Frame1d, colors}};
//! use device_envoy::led_strip;
//!
//! // Define LedStripSimple, a struct type for an 8-LED strip on PIN_0.
//! led_strip! {
//!     LedStripSimple {
//!         pin: PIN_0,  // GPIO pin for LED data
//!         len: 8,      // 8 LEDs
//!         // other inputs set to their defaults
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     // Create a LedStripSimple instance.
//!     let led_strip_simple = LedStripSimple::new(p.PIN_0, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     // Create and write a frame with alternating blue and gray pixels.
//!     let mut frame = Frame1d::new();
//!     for pixel_index in 0..LedStripSimple::LEN {
//!         // Directly index into the frame buffer.
//!         frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
//!     }
//!
//!     // Display the frame on the LED strip (until replaced).
//!     led_strip_simple.write_frame(frame)?;
//!
//!     core::future::pending().await // run forever
//! }
//! ```
//!
//! # Example: Animate a Sequence
//!
//! This example animates a 96-LED strip through red, green, and blue frames, cycling continuously.
//! Here, the generated struct type is named `LedStripAnimated`.
//!
//! ![LED strip preview][led_strip_animated]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::default::Default;
//! # use core::result::Result::Ok;
//! use device_envoy::{Result, led_strip::{Current, Frame1d, Gamma, colors}};
//! use device_envoy::led_strip;
//!
//! // Define LedStripAnimated, a struct type for a 96-LED strip on PIN_4.
//! // We change some defaults including setting a 1A power budget and disabling gamma correction.
//! led_strip! {
//!     pub(self) LedStripAnimated {               // Can provide a visibility modifier
//!         pin: PIN_4,                            // GPIO pin for LED data
//!         len: 96,                               // 96 LEDs
//!         pio: PIO1,                             // Use PIO resource 1
//!         dma: DMA_CH3,                          // Use DMA channel 3
//!         max_current: Current::Milliamps(1000), // 1A power budget
//!         gamma: Gamma::Linear,                  // No color correction
//!         max_frames: 3,                         // Up to 3 animation frames
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     let p = embassy_rp::init(Default::default());
//!     let led_strip_animated = LedStripAnimated::new(p.PIN_4, p.PIO1, p.DMA_CH3, spawner)?;
//!
//!     // Create a sequence of frames and durations and then animate them (looping, until replaced).
//!     let frame_duration = embassy_time::Duration::from_millis(300);
//!     led_strip_animated.animate([
//!         (Frame1d::filled(colors::RED), frame_duration),
//!         (Frame1d::filled(colors::GREEN), frame_duration),
//!         (Frame1d::filled(colors::BLUE), frame_duration),
//!     ])?;
//!
//!     core::future::pending().await // run forever
//! }
//! ```

/// 8-bit-per-channel RGB color re-exported from the
/// [`smart_leds`](https://docs.rs/smart-leds/latest/smart_leds/index.html) crate.
///
/// Used in [`Frame1d`] and [Frame2d](crate::led2d::Frame2d) for pixel colors.
///
/// See [`colors`] for the predefined color list.
///
/// Conversion to [`Rgb888`] via [`ToRgb888::to_rgb888`].
///
/// # [`smart_leds`](https://docs.rs/smart-leds/latest/smart_leds/type.RGB8.html) Documentation:
#[doc(inline)]
pub use smart_leds::RGB8;

/// Module containing predefined [`RGB8`] color constants, re-exported from the
/// [`smart_leds`](https://docs.rs/smart-leds/latest/smart_leds/index.html) crate.
///
/// These constants follow CSS/Web color names. In particular, `GREEN` is
/// `(0, 128, 0)` and `LIME` is `(0, 255, 0)`. If you want "full green," use
/// `LIME`.
///
/// All examples in this crate use `smart_leds::colors::*` as the single source
/// of named colors; when an embedded-graphics API needs [`Rgb888`], convert with
/// [`ToRgb888::to_rgb888`].
#[doc(inline)]
pub use smart_leds::colors;

/// 8-bit-per-channel RGB color re-exported from the
/// [`embedded-graphics`](https://docs.rs/embedded-graphics) crate.
///
/// See [Frame2d](crate::led2d::Frame2d) for usage examples of [`Rgb888`] and [`RGB8`].
///
/// Get named colors from [`colors`] and convert to `Rgb888` with
/// [`ToRgb888::to_rgb888`].
///
/// Conversion to [`RGB8`] via [`ToRgb8::to_rgb8`].
///
/// # [`embedded-graphics`](https://docs.rs/embedded-graphics/latest/embedded_graphics/pixelcolor/struct.Rgb888.html) Documentation:
#[doc(inline)]
pub use embedded_graphics::pixelcolor::Rgb888;

/// Convert colors to [`RGB8`] for LED strip rendering.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use core::assert_eq;
/// use device_envoy::led_strip::{Rgb888, ToRgb8, RGB8};
/// # fn main() {
/// let rgb8 = RGB8::new(16, 32, 48).to_rgb8();
/// let rgb888 = Rgb888::new(16, 32, 48);
/// let converted = rgb888.to_rgb8();
///
/// assert_eq!(rgb8, converted);
/// # }
/// ```
pub trait ToRgb8 {
    /// Convert this color to [`RGB8`].
    ///
    /// See the [`ToRgb8`](Self) trait docs for a usage example.
    #[must_use]
    fn to_rgb8(self) -> RGB8;
}

impl ToRgb8 for RGB8 {
    #[inline(always)]
    fn to_rgb8(self) -> RGB8 {
        self
    }
}

impl ToRgb8 for Rgb888 {
    #[inline(always)]
    fn to_rgb8(self) -> RGB8 {
        RGB8::new(self.r(), self.g(), self.b())
    }
}

/// Convert colors to [`Rgb888`] for embedded-graphics rendering.
///
/// # Example
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use core::assert_eq;
/// use device_envoy::led_strip::{Rgb888, ToRgb888, RGB8};
/// # fn main() {
/// let rgb8 = RGB8::new(16, 32, 48);
/// let rgb888 = rgb8.to_rgb888();
/// let already_rgb888 = Rgb888::new(16, 32, 48).to_rgb888();
///
/// assert_eq!(rgb888, already_rgb888);
/// # }
/// ```
pub trait ToRgb888 {
    /// Convert this color to [`Rgb888`].
    ///
    /// See the [`ToRgb888`](Self) trait docs for a usage example.
    #[must_use]
    fn to_rgb888(self) -> Rgb888;
}

impl ToRgb888 for RGB8 {
    #[inline(always)]
    fn to_rgb888(self) -> Rgb888 {
        Rgb888::new(self.r, self.g, self.b)
    }
}

impl ToRgb888 for Rgb888 {
    #[inline(always)]
    fn to_rgb888(self) -> Rgb888 {
        self
    }
}

#[cfg(not(feature = "host"))]
use core::borrow::Borrow;
use core::ops::{Deref, DerefMut};
use embedded_graphics::prelude::RgbColor;

// ============================================================================
// Gamma Correction
// ============================================================================

/// Gamma correction configuration for LED strips.
///
/// See the [`led_strip!`](macro@crate::led_strip), [`led_strips!`](crate::led_strips),
/// and [`led2d!`](mod@crate::led2d) macro docs for usage and context.
///
/// For background on gamma correction, see the
/// [Wikipedia article on gamma correction](https://en.wikipedia.org/wiki/Gamma_correction).
///
/// This is not display-calibrated color management; it is a simple LED brightness curve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gamma {
    /// No correction; raw LED PWM values.
    Linear,
    /// Perceptual sRGB semantics (gamma 2.2).
    ///
    /// This preserves the intent of the named color constants.
    Srgb,
    /// Compatibility with the historical `smart_leds::gamma()` curve (2.8).
    SmartLeds,
}

impl Default for Gamma {
    fn default() -> Self {
        Self::Srgb
    }
}

// Public so led_strip!/led_strips! expansions in downstream crates can reference it.
#[doc(hidden)]
/// Default gamma correction curve for generated LED devices (`Gamma::Srgb`).
pub const GAMMA_DEFAULT: Gamma = Gamma::Srgb;

/// Gamma 2.2 lookup table for 8-bit values.
/// Pre-computed to avoid floating point math: corrected = (value/255)^2.2 * 255
pub(crate) const GAMMA_SRGB_TABLE: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2,
    3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11,
    11, 12, 12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 22, 22, 23,
    23, 24, 25, 25, 26, 26, 27, 28, 28, 29, 30, 30, 31, 32, 33, 33, 34, 35, 35, 36, 37, 38, 39, 39,
    40, 41, 42, 43, 43, 44, 45, 46, 47, 48, 49, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61,
    62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 81, 82, 83, 84, 85, 87, 88,
    89, 90, 91, 93, 94, 95, 97, 98, 99, 100, 102, 103, 105, 106, 107, 109, 110, 111, 113, 114, 116,
    117, 119, 120, 121, 123, 124, 126, 127, 129, 130, 132, 133, 135, 137, 138, 140, 141, 143, 145,
    146, 148, 149, 151, 153, 154, 156, 158, 159, 161, 163, 165, 166, 168, 170, 172, 173, 175, 177,
    179, 181, 182, 184, 186, 188, 190, 192, 194, 196, 197, 199, 201, 203, 205, 207, 209, 211, 213,
    215, 217, 219, 221, 223, 225, 227, 229, 231, 234, 236, 238, 240, 242, 244, 246, 248, 251, 253,
    255,
];

/// Gamma 2.8 lookup table for 8-bit values.
/// Matches `smart_leds::gamma()` behavior.
pub(crate) const GAMMA_SMARTLEDS_TABLE: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

/// Linear lookup table (identity function).
const LINEAR_TABLE: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];

/// Generate a combined gamma correction and brightness scaling lookup table.
///
/// This combines two operations into a single table lookup for efficiency:
/// 1. Apply gamma correction based on the `gamma` parameter
/// 2. Scale by `max_brightness` for electrical current limiting
///
/// The result is a table where `combo_table[input_value]` gives the final output value.
#[doc(hidden)] // Implementation detail used by macro-generated strip types
#[must_use]
pub const fn generate_combo_table(gamma: Gamma, max_brightness: u8) -> [u8; 256] {
    let gamma_table = match gamma {
        Gamma::Linear => &LINEAR_TABLE,
        Gamma::Srgb => &GAMMA_SRGB_TABLE,
        Gamma::SmartLeds => &GAMMA_SMARTLEDS_TABLE,
    };
    let mut result = [0u8; 256];
    let mut index = 0;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this while loop with a for loop.
    while index < 256 {
        let gamma_corrected = gamma_table[index];
        // Apply brightness scaling: (value * brightness) / 255
        let scaled = ((gamma_corrected as u16 * max_brightness as u16) / 255) as u8;
        result[index] = scaled;
        index += 1;
    }
    result
}

#[cfg(not(feature = "host"))]
use core::cell::RefCell;
#[cfg(not(feature = "host"))]
use embassy_futures::select::{Either, select};
#[cfg(not(feature = "host"))]
use embassy_rp::pio::{Common, Instance};
#[cfg(not(feature = "host"))]
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
#[cfg(not(feature = "host"))]
use embassy_sync::blocking_mutex::Mutex;
#[cfg(not(feature = "host"))]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(not(feature = "host"))]
#[cfg(not(feature = "host"))]
use embassy_sync::once_lock::OnceLock;
#[cfg(not(feature = "host"))]
use embassy_sync::signal::Signal;
#[cfg(not(feature = "host"))]
use embassy_time::{Duration, Timer};
#[cfg(not(feature = "host"))]
use heapless::Vec;

#[cfg(not(feature = "host"))]
use crate::Result;

// ============================================================================
// Submodules
// ============================================================================

pub mod led_strip_generated;

/// 1D pixel array used to describe LED strip patterns.
///
/// See the [led_strip module documentation](mod@crate::led_strip) for usage examples.
///
/// Frames deref to `[RGB8; N]`, so you can mutate pixels directly before passing them to the generated strip's `write_frame` method.
#[derive(Clone, Copy, Debug)]
pub struct Frame1d<const N: usize>(pub [RGB8; N]);

impl<const N: usize> Frame1d<N> {
    /// Number of LEDs in this frame.
    pub const LEN: usize = N;

    /// Create a new blank (all black) frame.
    ///
    /// See the [led_strip module documentation](mod@crate::led_strip) for usage examples.
    #[must_use]
    pub const fn new() -> Self {
        Self([RGB8::new(0, 0, 0); N])
    }

    /// Create a frame filled with a single color.
    ///
    /// See the [led_strip module documentation](mod@crate::led_strip) for usage examples.
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

// ============================================================================
// PIO Bus - Shared PIO resource for multiple LED strips
// ============================================================================

/// A state machine bundled with its PIO bus.
///
/// This is returned by `pio_split!` and passed to strip constructors.
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioBusStateMachine<PIO: Instance + 'static, const SM: usize> {
    pio_bus: &'static PioBus<'static, PIO>,
    state_machine: embassy_rp::pio::StateMachine<'static, PIO, SM>,
}

#[cfg(not(feature = "host"))]
impl<PIO: Instance + 'static, const SM: usize> PioBusStateMachine<PIO, SM> {
    #[doc(hidden)]
    pub fn new(
        pio_bus: &'static PioBus<'static, PIO>,
        state_machine: embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) -> Self {
        Self {
            pio_bus,
            state_machine,
        }
    }

    #[doc(hidden)]
    pub fn pio_bus(&self) -> &'static PioBus<'static, PIO> {
        self.pio_bus
    }

    #[doc(hidden)]
    pub fn into_parts(
        self,
    ) -> (
        &'static PioBus<'static, PIO>,
        embassy_rp::pio::StateMachine<'static, PIO, SM>,
    ) {
        (self.pio_bus, self.state_machine)
    }
}

/// Shared PIO bus that manages the Common resource and WS2812 program.
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Support type for macro-generated strip types; not intended as surface API
pub struct PioBus<'d, PIO: Instance> {
    common: Mutex<CriticalSectionRawMutex, RefCell<Common<'d, PIO>>>,
    ws2812_program: OnceLock<PioWs2812Program<'d, PIO>>,
}

#[cfg(not(feature = "host"))]
impl<'d, PIO: Instance> PioBus<'d, PIO> {
    /// Create a new PIO bus with the given Common resource
    pub fn new(common: Common<'d, PIO>) -> Self {
        Self {
            common: Mutex::new(RefCell::new(common)),
            ws2812_program: OnceLock::new(),
        }
    }

    /// Get or initialize the WS2812 program (only loaded once)
    pub fn get_program(&'static self) -> &'static PioWs2812Program<'d, PIO> {
        self.ws2812_program.get_or_init(|| {
            self.common.lock(|common_cell: &RefCell<Common<'d, PIO>>| {
                let mut common = common_cell.borrow_mut();
                PioWs2812Program::new(&mut *common)
            })
        })
    }

    /// Access the common resource for initializing a driver
    pub fn with_common<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Common<'d, PIO>) -> R,
    {
        self.common.lock(|common_cell: &RefCell<Common<'d, PIO>>| {
            let mut common = common_cell.borrow_mut();
            f(&mut *common)
        })
    }
}

// ============================================================================
// LED Strip Command Channel and Static
// ============================================================================

#[cfg(not(feature = "host"))]
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub type LedStripCommandSignal<const N: usize, const MAX_FRAMES: usize> =
    Signal<CriticalSectionRawMutex, Command<N, MAX_FRAMES>>;

#[cfg(not(feature = "host"))]
#[cfg(not(feature = "host"))]
#[doc(hidden)]
// Command for the LED strip animation loop.
#[derive(Clone)]
pub enum Command<const N: usize, const MAX_FRAMES: usize> {
    DisplayStatic(Frame1d<N>),
    Animate(Vec<(Frame1d<N>, Duration), MAX_FRAMES>),
}

/// Static used to construct LED strip instances with animation support.
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Must be pub for method signatures and macro expansion in downstream crates
pub struct LedStripStatic<const N: usize, const MAX_FRAMES: usize> {
    command_signal: LedStripCommandSignal<N, MAX_FRAMES>,
}

#[cfg(not(feature = "host"))]
impl<const N: usize, const MAX_FRAMES: usize> LedStripStatic<N, MAX_FRAMES> {
    /// Creates static resources.
    #[must_use]
    #[doc(hidden)]
    pub const fn new_static() -> Self {
        Self {
            command_signal: Signal::new(),
        }
    }

    #[doc(hidden)]
    pub fn command_signal(&'static self) -> &'static LedStripCommandSignal<N, MAX_FRAMES> {
        &self.command_signal
    }
}

// Public so macro-generated types can deref to it; hidden from docs.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
/// Internal deref target for generated LED strip types.
///
/// All LED strip methods are available through macro-generated types.
/// See [`led_strip!`] macro documentation for usage.
pub struct LedStrip<const N: usize, const MAX_FRAMES: usize> {
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
}

#[cfg(not(feature = "host"))]
impl<const N: usize, const MAX_FRAMES: usize> LedStrip<N, MAX_FRAMES> {
    /// Creates LED strip resources.
    #[must_use]
    #[doc(hidden)]
    pub const fn new_static() -> LedStripStatic<N, MAX_FRAMES> {
        LedStripStatic::new_static()
    }

    /// Creates a new LED strip controller bound to the given static resources.
    pub fn new(led_strip_static: &'static LedStripStatic<N, MAX_FRAMES>) -> Result<Self> {
        Ok(Self {
            command_signal: led_strip_static.command_signal(),
        })
    }

    /// Writes a full frame to the LED strip. It remains displayed until another command
    /// replaces it.
    ///
    /// See the [led_strip module documentation](mod@crate::led_strip) for example usage.
    pub fn write_frame(&self, frame: Frame1d<N>) -> Result<()> {
        self.command_signal.signal(Command::DisplayStatic(frame));
        Ok(())
    }

    /// Loop forever through a sequence of animation frames.
    /// They remain displayed until another command replaces them.
    ///
    /// Each frame is a tuple of `(Frame1d, Duration)`. Accepts arrays, `Vec`s, or any
    /// iterator that produces `(Frame1d, Duration)` tuples.
    ///
    /// Returns immediately; the animation runs in the background until interrupted
    /// by a new `animate` call or `write_frame`.
    ///
    /// This uses [`embassy_time::Duration`] for frame timing.
    /// See the [led_strip module documentation](mod@crate::led_strip) for example usage.
    pub fn animate<I>(&self, frames: I) -> Result<()>
    where
        I: IntoIterator,
        I::Item: Borrow<(Frame1d<N>, embassy_time::Duration)>,
    {
        if MAX_FRAMES == 0 {
            return Err(crate::Error::AnimationDisabled(MAX_FRAMES));
        }
        let mut sequence: Vec<(Frame1d<N>, Duration), MAX_FRAMES> = Vec::new();
        for frame in frames {
            let (frame, duration) = *frame.borrow();
            assert!(
                duration.as_micros() > 0,
                "animation frame duration must be positive"
            );
            sequence
                .push((frame, duration))
                .expect("animation sequence fits within MAX_FRAMES");
        }
        self.animate_frames(sequence)
    }

    pub(crate) fn animate_frames(
        &self,
        sequence: Vec<(Frame1d<N>, Duration), MAX_FRAMES>,
    ) -> Result<()> {
        if MAX_FRAMES == 0 {
            return Err(crate::Error::AnimationDisabled(MAX_FRAMES));
        }
        assert!(
            !sequence.is_empty(),
            "animation requires at least one frame"
        );
        self.command_signal.signal(Command::Animate(sequence));
        Ok(())
    }
}

#[cfg(not(feature = "host"))]
#[doc(hidden)] // Required pub for macro expansion in downstream crates
pub async fn led_strip_device_loop<
    PIO,
    const SM: usize,
    const N: usize,
    const MAX_FRAMES: usize,
    ORDER,
>(
    mut driver: PioWs2812<'static, PIO, SM, N, ORDER>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> !
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    loop {
        let mut command = command_signal.wait().await;
        command_signal.reset();

        loop {
            match command {
                Command::DisplayStatic(mut frame) => {
                    apply_correction(&mut frame, combo_table);
                    driver.write(&frame).await;
                    break;
                }
                Command::Animate(frames) => {
                    command =
                        run_frame_animation(&mut driver, frames, command_signal, combo_table).await;
                }
            }
        }
    }
}

#[cfg(not(feature = "host"))]
async fn run_frame_animation<PIO, const SM: usize, const N: usize, const MAX_FRAMES: usize, ORDER>(
    driver: &mut PioWs2812<'static, PIO, SM, N, ORDER>,
    mut frames: Vec<(Frame1d<N>, Duration), MAX_FRAMES>,
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> Command<N, MAX_FRAMES>
where
    PIO: Instance,
    ORDER: embassy_rp::pio_programs::ws2812::RgbColorOrder,
{
    frames
        .iter_mut()
        .for_each(|(frame, _)| apply_correction(frame, combo_table));

    loop {
        for (frame, duration) in &frames {
            driver.write(frame).await;

            match select(command_signal.wait(), Timer::after(*duration)).await {
                Either::First(new_command) => {
                    command_signal.reset();
                    return new_command;
                }
                Either::Second(()) => continue,
            }
        }
    }
}

#[cfg(not(feature = "host"))]
fn apply_correction<const N: usize>(frame: &mut Frame1d<N>, combo_table: &[u8; 256]) {
    frame.iter_mut().for_each(|pixel| {
        pixel.r = combo_table[pixel.r as usize];
        pixel.g = combo_table[pixel.g as usize];
        pixel.b = combo_table[pixel.b as usize];
    });
}
/// Macro to generate multiple LED strip and panel struct types that share a single
/// [PIO resource](crate#glossary) (includes syntax details).
///
/// This page provides the primary documentation and examples for configuring strip/panel
/// groups that share a PIO resource.
///
/// **Syntax:**
///
/// ```text
/// led_strips! {
///     pio: <pio_ident>, // optional
///     [<visibility>] <GroupName> {
///         <MemberName>: {
///             pin: <pin_ident>,
///             len: <usize_expr>,
///             max_current: <Current_expr>,
///             gamma: <Gamma_expr>,          // optional
///             max_frames: <usize_expr>,     // optional
///             dma: <dma_ident>,             // optional
///             led2d: {                      // optional (panel mode)
///                 led_layout: <LedLayout_expr>,
///                 font: <Led2dFont_expr>,
///             }
///         },
///         // ...more members...
///     }
/// }
/// ```
///
/// **After reading the examples below, see also:**
///
/// - [`LedStripGenerated`](led_strip_generated::LedStripGenerated) — Sample LED **strip** type showing all methods and associated constants
/// - [`Led2dGenerated`](crate::led2d::led2d_generated::Led2dGenerated) — Sample LED **panel** type showing all methods and associated constants
/// - [`led_strip!`](macro@crate::led_strip) — Alternative macro to generate a single LED strip type. Consumes a PIO resource.
/// - [`led2d!`](mod@crate::led2d) — Alternative macro to generate a single LED panel type. Consumes a PIO resource.
///
/// Use this macro when your project has multiple LED strips or panels
/// that should share a single PIO resource.
/// If you only need a single strip or panel, prefer [`led_strip!`](macro@crate::led_strip)
/// or [`led2d!`](macro@crate::led2d) for simpler configuration.
///
/// We’ll start with a complete example below, then describe the required and
/// optional fields in detail.
///
/// # Example: Connect Three LED Strips/Panels to One PIO Resource
///
/// This example creates three LED strips/panels on GPIO0, GPIO3, and GPIO4,
/// all sharing PIO0. It demonstrates showing a pattern on the first two strips
/// and animating text on the 2D panel.
///
/// ![GPIO0 strip preview][led_strip_gpio0]
///
/// ![GPIO3 strip preview][led_strip_simple]
///
/// ![GPIO4 panel preview][led_strip_gogo]
///
/// ```rust,no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// # use core::convert::Infallible;
/// # use core::future;
/// # use defmt_rtt as _;
/// # use embassy_executor::Spawner;
/// # use defmt::info;
/// use device_envoy::{Result, led2d::Frame2d, led2d::Led2dFont, led2d::layout::LedLayout, led_strip::{Current, Frame1d, Gamma, colors, led_strips}};
/// use embassy_time::Duration;
///
/// // Our 2D panel is two 12x4 panels stacked vertically.
/// const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
/// const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
/// const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
///
/// led_strips! {
///     pio: PIO0,                          // Optional; defaults to PIO0.
///     LedStrips0 {                        // Name for this group of LED strips/panels. Can provide visibility modifier
///         // 1. a 8-LED strip on GPIO0
///         Gpio0LedStrip: {                // Exact struct name for this strip.
///             pin: PIN_0,                 // GPIO pin for LED data signal.
///             len: 8,                     // 8 LEDs on this strip.
///             max_current: Current::Milliamps(25), // Every strip/panel requires an electrical current budget.
///         },
///         // 2. a 48-LED strip on GPIO3
///         Gpio3LedStrip: {
///             pin: PIN_3,
///             len: 48,
///             max_current: Current::Milliamps(75),
///             gamma: Gamma::Srgb,        // Optional; color correction (default, Gamma::Srgb).
///             max_frames: 1,              // Optional; default 16 frames.
///             dma: DMA_CH11,              // Optional; auto-assigned by strip order.
///         },
///         // 3. a 96-LED 2D panel on GPIO4
///         Gpio4Led2d: {
///             pin: PIN_4,
///             len: 96,
///             max_current: Current::Milliamps(250),
///             max_frames: 2,
///             led2d: {                    // Optional panel configuration for 2D displays.
///                 led_layout: LED_LAYOUT_12X8_ROTATED, // Two 12×4 panels stacked and rotated.
///                 font: Led2dFont::Font4x6Trim, // 4x6 pixel font without the usual 1 pixel spacing.
///             }
///         },
///     }
/// }
///
/// # #[embassy_executor::main]
/// # async fn main(spawner: Spawner) -> ! {
/// #     let _ = example(spawner).await;
/// #     core::panic!("done");
/// # }
/// async fn example(spawner: Spawner) -> Result<Infallible> {
///     let p = embassy_rp::init(Default::default());
///
///     // Create instances of two LED strips and one panel.
///     let (gpio0_led_strip, gpio3_led_strip, gpio4_led2d) = LedStrips0::new(
///         p.PIO0, p.PIN_0, p.DMA_CH0, p.PIN_3, p.DMA_CH11, p.PIN_4, p.DMA_CH2, spawner,
///     )?;
///
///     info!("Setting GPIO0 to white, GPIO3 to alternating blue/gray, GPIO4 to Go Go animation");
///
///     // Turn on all-white on GPIO0 strip.
///     let frame_gpio0 = Frame1d::filled(colors::WHITE);
///     gpio0_led_strip.write_frame(frame_gpio0)?; // Display the frame (until replaced)
///
///     // Alternate blue/gray on GPIO3 strip.
///     let mut frame_gpio3 = Frame1d::new();
///     for pixel_index in 0..Gpio3LedStrip::LEN {
///         frame_gpio3[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
///     }
///     gpio3_led_strip.write_frame(frame_gpio3)?;  // Display the frame (until replaced)
///
///     // Animate "Go Go" text on GPIO4 2D panel.
///     let mut frame_go_top = Frame2d::new();
///     gpio4_led2d.write_text_to_frame("Go", &[], &mut frame_go_top)?;
///
///     let mut frame_go_bottom = Frame2d::new();
///     gpio4_led2d.write_text_to_frame(
///         "\nGo",
///         &[colors::HOT_PINK, colors::LIME],
///         &mut frame_go_bottom,
///     )?;
///
///     let frame_duration = Duration::from_secs(1);
///     gpio4_led2d
///         .animate([
///             (frame_go_top, frame_duration),
///             (frame_go_bottom, frame_duration),
///         ])?; // Loop animation (until replaced)
///
///     future::pending::<Result<Infallible>>().await // Run forever
/// }
/// ```
///
/// # Configuration
///
/// ## Shared PIO Resource
///
/// - `pio` — PIO peripheral to use (default: `PIO0`). This consumes one PIO resource for the
///   group and is shared across all strips/panels in the macro invocation.
///
/// ## Required Fields per Strip/Panel
///
/// - `pin` — GPIO pin for LED data
/// - `len` — Number of LEDs (pixels)
/// - `max_current` — Electrical current budget per strip (required; no default)
///
/// ## Optional Fields per Strip/Panel
///
/// - `dma` — DMA channel (default: auto-assigned by strip order)
/// - `gamma` — Gamma correction curve (default: `Gamma::Srgb`)
/// - `max_frames` — Maximum number of animation frames (default: 16 frames)
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
/// - `led2d` — Marks this strip as a 2D LED panel and enables 2D rendering support (optional, see below).
///    Detailed 2D rendering, examples, and animation support are documented
///    in the [`led2d` module](mod@crate::led2d).
///
/// ## 2D Panel Configuration (`led2d`)
///
/// If a strip represents a rectangular LED panel rather than a linear strip,
/// add a `led2d` configuration block to describe its geometry.
///
/// Required fields:
/// - `led_layout` — Physical layout mapping (defines the panel size). See [`LedLayout`](crate::led2d::layout::LedLayout) for details.
/// - `font` — Built-in font for text rendering. See [`Led2dFont`](crate::led2d::Led2dFont) for available fonts.
///   Bring `Led2dFont` into scope or use a full path like `device_envoy::led2d::Led2dFont::Font4x6Trim`.
///
/// The `led_layout` value must be a const so its dimensions can be derived at compile time.
///
/// # Capacity and Board Capabilities
///
/// The `led_strips!` macro is designed to **fully utilize the PIO resources**
/// of supported Pico boards.
///
/// Each `led_strips!` invocation can drive up to **4 LED strips or panels**
/// while sharing a single PIO resource. This lets you consolidate multiple
/// LED outputs efficiently instead of consuming one PIO per strip.
///
/// Each invocation consumes exactly one PIO resource.
///
/// On supported boards, this enables the maximum practical LED capacity:
///
/// - **Pico 1** provides **2 PIO resources**, allowing up to **8 LED strips or panels**
/// - **Pico 2** provides **3 PIO resources**, allowing up to **12 LED strips or panels**
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led_strip!`](macro@crate::led_strip) — For a single 1-dimensional LED strip (includes examples)
/// - [`led2d!`](mod@crate::led2d) — For 2-dimensional LED panels
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gpio0",
        "docs/assets/led_strip_gpio0.png"
    )
)]
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_simple",
        "docs/assets/led_strip_simple.png"
    )
)]
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_gogo",
        "docs/assets/led2d2.png"
    )
)]
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! led_strips {
    ($($tt:tt)*) => { $crate::__led_strips_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led_strips!`] instead.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strips_impl {
    // Internal: full expansion with all fields specified
    (@__expand
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips: [
            $(
                $label:ident {
                    sm: $sm_index:expr,
                    dma: $dma:ident,
                    pin: $pin:ident,
                    len: $len:expr,
                    max_current: $max_current:expr,
                    gamma: $gamma:expr,
                    max_frames: $max_frames:expr
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                            $(max_frames: $led2d_max_frames:expr,)?
                            font: $led2d_font:expr $(,)?
                        }
                    )?
                }
            ),+ $(,)?
        ]
    ) => {
        // Use crate-level PIO interrupt bindings (Pio0Irqs, Pio1Irqs, Pio2Irqs)
        paste::paste! {
            // Create the PIO bus
            #[allow(non_upper_case_globals)]
            static [<$pio _BUS>]: ::static_cell::StaticCell<
                $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>
            > = ::static_cell::StaticCell::new();

            /// Split the PIO into bus and state machines.
            ///
            /// Returns 4 StateMachines (one for each SM)
            #[allow(dead_code)]
            pub fn [<$pio:lower _split>](
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
            ) -> (
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 0>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 1>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 2>,
                $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 3>,
            ) {
                let ::embassy_rp::pio::Pio { common, sm0, sm1, sm2, sm3, .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio,
                        <::embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                let pio_bus = [<$pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                (
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm0),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm1),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm2),
                    $crate::led_strip::PioBusStateMachine::new(pio_bus, sm3),
                )
                }

            // Create strip types
        $(
            $crate::__led_strips_impl!(
                @__define_strip
                vis: $vis,
                group: $group,
                pio: $pio,
                label: $label,
                sm: $sm_index,
                dma: $dma,
                pin: $pin,
                len: $len,
                max_current: $max_current,
                gamma: $gamma,
                max_frames: $max_frames
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        $(max_frames: $led2d_max_frames,)?
                        font: $led2d_font,
                    }
                )?
            );
        )+

            // Generate the group marker struct with new() constructor
            #[allow(missing_docs)]
            $vis struct $group;

            #[allow(missing_docs)]
            impl $group {
                #[allow(clippy::too_many_arguments)]
                pub fn new(
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    $(
                        [<$label:snake _pin>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                        [<$label:snake _dma>]: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    )+
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<(
                    $(
                        $crate::__led_strips_impl!(
                            @__strip_return_type
                            $label
                            $(,
                                led2d: {
                                    led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                                    font: $led2d_font,
                                }
                            )?
                        ),
                    )+
                )> {
                    // Inline PIO splitting
                    let pio_peri = pio.into();
                    let ::embassy_rp::pio::Pio { common, sm0, sm1, sm2, sm3, .. } =
                        ::embassy_rp::pio::Pio::new(
                            pio_peri,
                            <::embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                        );
                    let pio_bus = [<$pio _BUS>].init_with(|| {
                        $crate::led_strip::PioBus::new(common)
                    });

                    // Create individual state machine wrappers
                    #[allow(unused_variables)]
                    let sm0_wrapped = $crate::led_strip::PioBusStateMachine::new(pio_bus, sm0);
                    #[allow(unused_variables)]
                    let sm1_wrapped = $crate::led_strip::PioBusStateMachine::new(pio_bus, sm1);
                    #[allow(unused_variables)]
                    let sm2_wrapped = $crate::led_strip::PioBusStateMachine::new(pio_bus, sm2);
                    #[allow(unused_variables)]
                    let sm3_wrapped = $crate::led_strip::PioBusStateMachine::new(pio_bus, sm3);

                    // Construct each strip with the appropriate SM
                    Ok((
                        $(
                            $crate::__led_strips_impl!(
                                @__strip_return_value
                                label: $label,
                                state_machine: $crate::__led_strips_impl!(
                                    @__select_sm
                                    $sm_index,
                                    sm0_wrapped,
                                    sm1_wrapped,
                                    sm2_wrapped,
                                    sm3_wrapped
                                ),
                                pin: [<$label:snake _pin>],
                                dma: [<$label:snake _dma>],
                                spawner: spawner
                                $(,
                                    led2d: {
                                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                                        font: $led2d_font,
                                    }
                                )?
                            ),
                        )+
                    ))
                }
            }
        }
    };

    // Helper to select the right SM based on index
    (@__select_sm 0, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm0 };
    (@__select_sm 1, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm1 };
    (@__select_sm 2, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm2 };
    (@__select_sm 3, $sm0:ident, $sm1:ident, $sm2:ident, $sm3:ident) => { $sm3 };

    (@__define_strip
        vis: $vis:vis,
        group: $group:ident,
        pio: $pio:ident,
        label: $label:ident,
        sm: $sm_index:expr,
        dma: $dma:ident,
        pin: $pin:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr
    ) => {
        paste::paste! {
            #[doc = concat!(
                "LED strip wrapper generated by [`led_strips!`].\n\n",
                "Derefs to provide all LED control methods. ",
                "Created with [`", stringify!($group), "::new`]. ",
                "See the [led_strip module documentation](mod@crate::led_strip) for a similar example."
            )]
            $vis struct $label {
                strip: $crate::led_strip::LedStrip<{ $len }, { $max_frames }>,
            }

            #[allow(missing_docs)]
            impl $label {
                pub const LEN: usize = $len;
                pub const MAX_FRAMES: usize = $max_frames;

                // Calculate max brightness from current budget
                // Each WS2812B LED draws ~60mA at full brightness
                const WORST_CASE_MA: u32 = ($len as u32) * 60;
                pub const MAX_BRIGHTNESS: u8 =
                    $max_current.max_brightness(Self::WORST_CASE_MA);

                // Combined gamma correction and brightness scaling table
                const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                pub(crate) const fn new_static() -> $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> {
                    $crate::led_strip::LedStrip::new_static()
                }

                pub fn new(
                    state_machine: $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> =
                        $label::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<$label> = ::static_cell::StaticCell::new();
                    let pin = pin.into();
                    let dma = dma.into();

                    let (bus, sm) = state_machine.into_parts();
                    let token = [<$label:snake _device_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                    );
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                    let strip = $crate::led_strip::LedStrip::new(&STRIP_STATIC)?;
                    let instance = STRIP_CELL.init(Self { strip });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for $label {
                type Target = $crate::led_strip::LedStrip<{ $len }, { $max_frames }>;

                fn deref(&self) -> &Self::Target {
                    &self.strip
                }
            }

            #[cfg(not(feature = "host"))]
            impl AsRef<$crate::led_strip::LedStrip<{ $len }, { $max_frames }>> for $label {
                fn as_ref(&self) -> &$crate::led_strip::LedStrip<{ $len }, { $max_frames }> {
                    &self.strip
                }
            }

            #[::embassy_executor::task]
            async fn [<$label:snake _device_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, $sm_index>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        $sm_index,
                        { $len },
                        _
                    >::new(common, sm, dma, pin, program)
                });
                $crate::led_strip::led_strip_device_loop::<
                    ::embassy_rp::peripherals::$pio,
                    $sm_index,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, &$label::COMBO_TABLE).await
            }
        }
    };

    (@__define_strip
        vis: $vis:vis,
        group: $group:ident,
        pio: $pio:ident,
        label: $label:ident,
        sm: $sm_index:expr,
        dma: $dma:ident,
        pin: $pin:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: {
            led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
            $(max_frames: $led2d_max_frames:expr,)?
            font: $led2d_font:expr $(,)?
        }
    ) => {
        paste::paste! {
            #[doc = concat!(
                "LED strip wrapper generated by [`led_strips!`].\n\n",
                "Derefs to provide all LED control methods. ",
                "Created with [`", stringify!($group), "::new`]. ",
                "See the [led_strip module documentation](mod@crate::led_strip) for a similar example."
            )]
            #[allow(missing_docs)]
            struct [<$label:camel LedStrip>] {
                strip: $crate::led_strip::LedStrip<{ $len }, { $max_frames }>,
            }

            #[allow(missing_docs)]
            impl [<$label:camel LedStrip>] {
                pub const LEN: usize = $len;
                pub const MAX_FRAMES: usize = $max_frames;

                // Calculate max brightness from current budget
                // Each WS2812B LED draws ~60mA at full brightness
                const WORST_CASE_MA: u32 = ($len as u32) * 60;
                pub const MAX_BRIGHTNESS: u8 =
                    $max_current.max_brightness(Self::WORST_CASE_MA);

                // Combined gamma correction and brightness scaling table
                const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                pub(crate) const fn new_static() -> $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> {
                    $crate::led_strip::LedStrip::new_static()
                }

                pub fn new(
                    state_machine: $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> =
                        [<$label:camel LedStrip>]::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<[<$label:camel LedStrip>]> = ::static_cell::StaticCell::new();
                    let pin = pin.into();
                    let dma = dma.into();

                    let (bus, sm) = state_machine.into_parts();
                    let token = [<$label:snake _led_strip _device_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                    );
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                    let strip = $crate::led_strip::LedStrip::new(&STRIP_STATIC)?;
                    let instance = STRIP_CELL.init(Self { strip });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for [<$label:camel LedStrip>] {
                type Target = $crate::led_strip::LedStrip<{ $len }, { $max_frames }>;

                fn deref(&self) -> &Self::Target {
                    &self.strip
                }
            }

            #[cfg(not(feature = "host"))]
            impl AsRef<$crate::led_strip::LedStrip<{ $len }, { $max_frames }>> for [<$label:camel LedStrip>] {
                fn as_ref(&self) -> &$crate::led_strip::LedStrip<{ $len }, { $max_frames }> {
                    &self.strip
                }
            }

            #[::embassy_executor::task]
            async fn [<$label:snake _led_strip _device_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, $sm_index>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        $sm_index,
                        { $len },
                        _
                    >::new(common, sm, dma, pin, program)
                });
                $crate::led_strip::led_strip_device_loop::<
                    ::embassy_rp::peripherals::$pio,
                    $sm_index,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, &[<$label:camel LedStrip>]::COMBO_TABLE).await
            }

            #[cfg(not(feature = "host"))]
            $crate::led2d::led2d_from_strip! {
                $vis $label,
                strip_type: [<$label:camel LedStrip>],
                width: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?.width(),
                height: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?.height(),
                led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                font: $led2d_font,
            }

            #[cfg(not(feature = "host"))]
            impl [<$label:camel LedStrip>] {
                pub fn new_led2d(
                    state_machine: $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, $sm_index>,
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<$label> {
                    let strip = Self::new(state_machine, pin, dma, spawner)?;
                    $label::from_strip(strip)
                }
            }
        }
    };

    (@__strip_return_type $label:ident) => {
        &'static $label
    };
    (@__strip_return_type $label:ident, led2d: { $($led2d_fields:tt)* }) => {
        $label
    };


    (@__strip_return_value
        label: $label:ident,
        state_machine: $state_machine:expr,
        pin: $pin:ident,
        dma: $dma:ident,
        spawner: $spawner:ident
    ) => {
        $label::new($state_machine, $pin, $dma, $spawner)?
    };
    (@__strip_return_value
        label: $label:ident,
        state_machine: $state_machine:expr,
        pin: $pin:ident,
        dma: $dma:ident,
        spawner: $spawner:ident,
        led2d: {
            led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
            $(max_frames: $led2d_max_frames:expr,)?
            font: $led2d_font:expr $(,)?
        }
    ) => {
        paste::paste! {{
            let [<$label:snake _led_strip>] =
                [<$label:camel LedStrip>]::new($state_machine, $pin, $dma, $spawner)?;
            $label::from_strip([<$label:snake _led_strip>])?
        }}
    };

    // Entry point with explicit pio and group syntax (internal use by led2d)
    (@__with_frame_alias
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    // Entry point with visibility and pio
    (
        pio: $pio:ident,
        $vis:vis $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: __WITH_FRAME_ALIAS__,
            pio: $pio,
            vis: $vis,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    // Entry point with pio, defaults to pub visibility - REMOVED (visibility required)

    // Entry point without pio (defaults to PIO0) with group syntax - REMOVED (visibility required)

    // Entry point with visibility, defaults to PIO0
    (
        $vis:vis $group:ident {
            $( $label:ident: { $($fields:tt)* } ),+ $(,)?
        }
    ) => {
        $crate::__led_strips_impl! {
            @__with_defaults
            frame_alias: __WITH_FRAME_ALIAS__,
            pio: PIO0,
            vis: $vis,
            group: $group,
            sm_counter: 0,
            strips_out: [],
            strips_in: [ $( $label: { $($fields)* } ),+ ]
        }
    };

    // Entry point without visibility, defaults to pub and PIO0 - REMOVED (visibility required)

    // Process strips one at a time, adding defaults
    (@__with_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_in: [ $label:ident: { $($fields:tt)* } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($($rest)*)? ],
            label: $label,
            group: $group,
            pin: __MISSING_PIN__,
            dma: __DEFAULT_DMA__,
            len: __MISSING_LEN__,
            max_current: __MISSING_MAX_CURRENT__,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            led2d: __NONE__,
            fields: [ $($fields)* ]
        }
    };

    // All strips processed, call the main implementation
    (@__with_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_in: []
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [],
            strips_in: [ $($out)* ]
        }
    };

    // Resolve any __DEFAULT_DMA__ placeholders before expansion.
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: []
    ) => {
        $crate::__led_strips_impl! {
            @__expand
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips: [ $($out)* ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 0,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        $(max_frames: $led2d_max_frames:expr,)?
                        font: $led2d_font:expr $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 0,
                    dma: DMA_CH0,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 0 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 0,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:expr $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 0,
                    dma: DMA_CH0,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 1 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 1,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:expr $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 1,
                    dma: DMA_CH1,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 1 with led2d and explicit max_frames (original)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 1,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        $(max_frames: $led2d_max_frames:expr,)?
                        font: $led2d_font:expr $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 1,
                    dma: DMA_CH1,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 2 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 2,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:expr $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 2,
                    dma: DMA_CH2,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    // SM 3 with led2d but no explicit max_frames (use strip-level max_frames)
    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 3,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                ,
                led2d: {
                    led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                    font: $led2d_font:expr $(,)?
                }
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 3,
                    dma: DMA_CH3,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    ,
                    led2d: {
                        led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                        max_frames: $max_frames,
                        font: $led2d_font,
                    }
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 2,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        $(max_frames: $led2d_max_frames:expr,)?
                        font: $led2d_font:expr $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 2,
                    dma: DMA_CH2,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: 3,
                dma: __DEFAULT_DMA__,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        $(max_frames: $led2d_max_frames:expr,)?
                        font: $led2d_font:expr $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: 3,
                    dma: DMA_CH3,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };

    (@__resolve_default_dma
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        group: $group:ident,
        strips_out: [ $($out:tt)* ],
        strips_in: [
            $label:ident {
                sm: $sm_index:expr,
                dma: $dma:ident,
                pin: $pin:ident,
                len: $len:expr,
                max_current: $max_current:expr,
                gamma: $gamma:expr,
                max_frames: $max_frames:expr
                $(,
                    led2d: {
                        led_layout: $led2d_led_layout:ident $( ( $($led2d_led_layout_args:tt)* ) )?,
                        $(max_frames: $led2d_max_frames:expr,)?
                        font: $led2d_font:expr $(,)?
                    }
                )?
            }
            $(, $($rest:tt)* )?
        ]
    ) => {
        $crate::__led_strips_impl! {
            @__resolve_default_dma
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm_index,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                    $(,
                        led2d: {
                            led_layout: $led2d_led_layout $( ( $($led2d_led_layout_args)* ) )?,
                            font: $led2d_font,
                        }
                    )?
                },
            ],
            strips_in: [ $($($rest)*)? ]
        }
    };


    // Parse fields for a single strip
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ pin: $new_pin:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $new_pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ dma: $new_dma:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $new_dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ len: $new_len:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $new_len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ max_current: $new_max_current:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $new_max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ gamma: $new_gamma:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $new_gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: [ max_frames: $new_max_frames:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $new_max_frames,
            led2d: $led2d,
            fields: [ $($($rest)*)? ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __NONE__,
        fields: [ led2d: { $($led2d_fields:tt)* } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: $sm,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: __HAS_LED2D__ { $($led2d_fields)* },
            fields: [ $($($rest)*)? ]
        }
    };

    // Done parsing fields, add strip to output and continue
    // Special case: convert __DEFAULT_DMA__ to actual DMA channel based on sm
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: 0,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: 0,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH0,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: 1,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: 1,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH1,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: 2,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: 2,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH2,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: 3,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: __DEFAULT_DMA__,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: $led2d:tt,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__fill_strip_defaults
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            sm_counter: 3,
            strips_out: [ $($out)* ],
            strips_remaining: [ $($remaining)* ],
            label: $label,
            group: $group,
            pin: $pin,
            dma: DMA_CH3,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            led2d: $led2d,
            fields: []
        }
    };

    // Done parsing fields, add strip to output and continue
    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: __MISSING_MAX_CURRENT__,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __NONE__,
        fields: []
    ) => {
        compile_error!("led_strips!: max_current is required for every strip");
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: __MISSING_MAX_CURRENT__,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __HAS_LED2D__ { $($led2d_fields:tt)* },
        fields: []
    ) => {
        compile_error!("led_strips!: max_current is required for every strip");
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __NONE__,
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__inc_counter
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            sm: $sm,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames
                },
            ],
            strips_in: [ $($remaining)* ]
        }
    };

    (@__fill_strip_defaults
        frame_alias: $frame_alias:tt,
        pio: $pio:ident,
        vis: $vis:vis,
        sm_counter: $sm:tt,
        strips_out: [ $($out:tt)* ],
        strips_remaining: [ $($remaining:tt)* ],
        label: $label:ident,
        group: $group:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        led2d: __HAS_LED2D__ { $($led2d_fields:tt)* },
        fields: []
    ) => {
        $crate::__led_strips_impl! {
            @__inc_counter
            frame_alias: $frame_alias,
            pio: $pio,
            vis: $vis,
            group: $group,
            sm: $sm,
            strips_out: [
                $($out)*
                $label {
                    sm: $sm,
                    dma: $dma,
                    pin: $pin,
                    len: $len,
                    max_current: $max_current,
                    gamma: $gamma,
                    max_frames: $max_frames,
                    led2d: { $($led2d_fields)* }
                },
            ],
            strips_in: [ $($remaining)* ]
        }
    };
    // Increment counter by expanding to literal numbers
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, vis: $vis:vis, group: $group:ident, sm: 0, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, vis: $vis, group: $group, sm_counter: 1, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, vis: $vis:vis, group: $group:ident, sm: 1, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, vis: $vis, group: $group, sm_counter: 2, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, vis: $vis:vis, group: $group:ident, sm: 2, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, vis: $vis, group: $group, sm_counter: 3, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
    (@__inc_counter frame_alias: $frame_alias:tt, pio: $pio:ident, vis: $vis:vis, group: $group:ident, sm: 3, strips_out: [$($out:tt)*], strips_in: [$($in:tt)*]) => {
        $crate::__led_strips_impl! { @__with_defaults frame_alias: $frame_alias, pio: $pio, vis: $vis, group: $group, sm_counter: 4, strips_out: [$($out)*], strips_in: [$($in)*] }
    };
}

/// Macro to generate an LED-strip struct type (includes syntax details). See
/// [`LedStripGenerated`](led_strip_generated::LedStripGenerated) for a sample of a generated type.
///
/// **See the [led_strip module documentation](mod@crate::led_strip) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// led_strip! {
///     [<visibility>] <Name> {
///         pin: <pin_ident>,
///         len: <usize_expr>,
///         pio: <pio_ident>,               // optional
///         dma: <dma_ident>,               // optional
///         max_current: <Current_expr>,    // optional
///         gamma: <Gamma_expr>,            // optional
///         max_frames: <usize_expr>,       // optional
///     }
/// }
/// ```
///
/// **Required fields:**
///
/// - `pin` — GPIO pin for LED data
/// - `len` — Number of LEDs
///
/// **Optional fields:**
///
/// - `pio` — PIO resource (default: `PIO0`)
/// - `dma` — DMA channel (default: `DMA_CH0`)
/// - `max_current` — Electrical current budget (default: 250 mA)
/// - `gamma` — Color curve (default: `Gamma::Srgb`)
/// - `max_frames` — Maximum number of animation frames (default: 16 frames)
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other strips or panels (includes examples)
/// - [`led2d!`](mod@crate::led2d) — For 2-dimensional LED panels
///
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! led_strip {
    // TODO_NIGHTLY When nightly feature `decl_macro` becomes stable, change this
    // code by replacing `#[macro_export] macro_rules!` with module-scoped `pub macro`
    // so macro visibility and helper exposure can be controlled more precisely.
    ($($tt:tt)*) => { $crate::__led_strip_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led_strip!`] instead.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_impl {
    // Entry point - name without visibility defaults to public
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: pub(self),
            pio: PIO0,
            name: $name,
            pin: _UNSET_,
            dma: DMA_CH0,
            len: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            fields: [ $($fields)* ]
        }
    };

    // Entry point - name with explicit visibility
    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: PIO0,
            name: $name,
            pin: _UNSET_,
            dma: DMA_CH0,
            len: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            fields: [ $($fields)* ]
        }
    };

    // Fill defaults: pio
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ pio: $new_pio:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $new_pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: pin
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ pin: $new_pin:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $new_pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: dma
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ dma: $new_dma:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $new_dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: len (expression in braces)
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ len: { $new_len:expr } $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: { $new_len },
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: len (plain expression)
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ len: $new_len:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $new_len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_current
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ max_current: $new_max_current:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $new_max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: gamma
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ gamma: $new_gamma:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $new_gamma,
            max_frames: $max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_frames
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        len: $len:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: [ max_frames: $new_max_frames:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $new_max_frames,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill default max_current if still unset
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: _UNSET_,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: []
    ) => {
        $crate::__led_strip_impl! {
            @__fill_defaults
            vis: $vis,
            pio: $pio,
            name: $name,
            pin: $pin,
            dma: $dma,
            len: $len,
            max_current: $crate::led_strip::MAX_CURRENT_DEFAULT,
            gamma: $gamma,
            max_frames: $max_frames,
            fields: []
        }
    };

    // All fields processed - expand the type
    (@__fill_defaults
        vis: $vis:vis,
        pio: $pio:ident,
        name: $name:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        len: $len:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        fields: []
    ) => {
        ::paste::paste! {
            // Create the PIO bus (shared across all SM0 strips using this PIO)
            #[allow(non_upper_case_globals)]
            static [<$name:snake _ $pio _BUS>]: ::static_cell::StaticCell<
                $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>
            > = ::static_cell::StaticCell::new();

            /// Split the PIO into bus and state machines.
            ///
            /// Returns SM0 only for single-strip usage.
            #[allow(dead_code)]
            fn [<$name:snake _split_sm0>](
                pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
            ) -> $crate::led_strip::PioBusStateMachine<::embassy_rp::peripherals::$pio, 0> {
                let ::embassy_rp::pio::Pio { common, sm0, .. } =
                    ::embassy_rp::pio::Pio::new(
                        pio,
                        <::embassy_rp::peripherals::$pio as $crate::pio_irqs::PioIrqMap>::irqs(),
                    );
                let pio_bus = [<$name:snake _ $pio _BUS>].init_with(|| {
                    $crate::led_strip::PioBus::new(common)
                });
                $crate::led_strip::PioBusStateMachine::new(pio_bus, sm0)
            }

            #[doc = concat!(
                "LED strip generated by [`led_strip!`] or [`led_strips!`](crate::led_strips!).\n\n",
                "See the [led_strip module documentation](mod@crate::led_strip) for usage and examples."
            )]
            $vis struct $name {
                strip: $crate::led_strip::LedStrip<{ $len }, { $max_frames }>,
            }

            impl $name {
                /// The number of LEDs in this strip (determined by the `len` parameter in [`led_strip!`] or [`led_strips!`](crate::led_strips!)).
                pub const LEN: usize = $len;
                /// Maximum number of animation frames (determined by `max_frames` parameter).
                pub const MAX_FRAMES: usize = $max_frames;

                // Calculate max brightness from current budget
                const WORST_CASE_MA: u32 = ($len as u32) * 60;
                /// Maximum brightness level, automatically limited by the power budget specified in `max_current`.
                /// We assume, each LED draws 60 mA at full brightness.
                pub const MAX_BRIGHTNESS: u8 =
                    $max_current.max_brightness(Self::WORST_CASE_MA);

                // Combined gamma correction and brightness scaling table
                const COMBO_TABLE: [u8; 256] = $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                /// Create a new LED strip instance of the struct type
                /// defined by [`led_strip!`] or [`led_strips!`](crate::led_strips!).
                ///
                /// See the [led_strip module documentation](mod@crate::led_strip) for example usage.
                ///
                /// The `pin`, `pio`, and `dma` parameters must correspond to the
                /// GPIO pin, PIO resource, and DMA channel specified in the macro.
                ///
                /// - The [`led_strip!`] macro defaults to `PIO0` and `DMA_CH0` if not specified.
                /// - The [`led_strips!`](crate::led_strips!) macro defaults to `PIO0` and
                /// automatically assigns consecutive DMA channels to each strip, starting at `DMA_CH0`.
                ///
                /// # Parameters
                ///
                /// - `pin`: GPIO pin for LED data signal
                /// - `pio`: PIO resource
                /// - `dma`: DMA channel for LED data transfer
                /// - `spawner`: Task spawner for background operations
                pub fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    static STRIP_STATIC: $crate::led_strip::LedStripStatic<{ $len }, { $max_frames }> =
                        $crate::led_strip::LedStrip::new_static();
                    static STRIP_CELL: ::static_cell::StaticCell<$name> = ::static_cell::StaticCell::new();

                    let pin = pin.into();
                    let dma = dma.into();

                    let sm0 = [<$name:snake _split_sm0>](pio);
                    let (bus, sm) = sm0.into_parts();

                    let token = [<$name:snake _device_task>](
                        bus,
                        sm,
                        dma,
                        pin,
                        STRIP_STATIC.command_signal(),
                    );
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;

                    let strip = $crate::led_strip::LedStrip::new(&STRIP_STATIC)?;
                    let instance = STRIP_CELL.init($name { strip });
                    Ok(instance)
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::led_strip::LedStrip<{ $len }, { $max_frames }>;

                fn deref(&self) -> &Self::Target {
                    &self.strip
                }
            }

            #[cfg(not(feature = "host"))]
            impl AsRef<$crate::led_strip::LedStrip<{ $len }, { $max_frames }>> for $name {
                fn as_ref(&self) -> &$crate::led_strip::LedStrip<{ $len }, { $max_frames }> {
                    &self.strip
                }
            }

            #[::embassy_executor::task]
            async fn [<$name:snake _device_task>](
                bus: &'static $crate::led_strip::PioBus<'static, ::embassy_rp::peripherals::$pio>,
                sm: ::embassy_rp::pio::StateMachine<'static, ::embassy_rp::peripherals::$pio, 0>,
                dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                command_signal: &'static $crate::led_strip::LedStripCommandSignal<{ $len }, { $max_frames }>,
            ) -> ! {
                let program = bus.get_program();
                let driver = bus.with_common(|common| {
                    ::embassy_rp::pio_programs::ws2812::PioWs2812::<
                        ::embassy_rp::peripherals::$pio,
                        0,
                        { $len },
                        _
                    >::new(common, sm, dma, pin, program)
                });
                $crate::led_strip::led_strip_device_loop
                ::<
                    ::embassy_rp::peripherals::$pio,
                    0,
                    { $len },
                    { $max_frames },
                    _
                >(driver, command_signal, &$name::COMBO_TABLE).await
            }
        }
    };
}

/// Macro for advanced PIO splitting (rarely needed).
///
/// **See the module docs.** Most users should use the group constructor instead.
#[doc(hidden)]
#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! pio_split {
    ($p:ident . PIO0) => {
        pio0_split($p.PIO0)
    };
    ($p:ident . PIO1) => {
        pio1_split($p.PIO1)
    };
    ($p:ident . PIO2) => {
        pio2_split($p.PIO2)
    };
}

#[cfg(not(feature = "host"))]
pub use pio_split;

#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use led_strip;
#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use led_strips;

/// Electrical current budget configuration for LED strips.
///
/// See the [`led_strip!`](macro@crate::led_strip), [`led_strips!`](crate::led_strips),
/// and [`led2d!`](mod@crate::led2d) macro docs for usage and context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Current {
    /// Limit brightness to stay within a specific milliamp budget.
    ///
    /// The `max_brightness` is automatically calculated to ensure the worst-case electrical current
    /// (all LEDs at full brightness) does not exceed this limit. For example, a 16-LED strip
    /// draws 960 mA at full brightness (assuming 60 mA per LED); with the default electrical current
    /// limit, brightness is capped at ~26%.
    ///
    /// See the [`led_strip!`](macro@crate::led_strip), [`led_strips!`](crate::led_strips),
    /// and [`led2d!`](mod@crate::led2d) macro docs for usage and context.
    Milliamps(u16),
    /// No limit — brightness stays at 100% (subject to practical hardware constraints like
    /// USB power delivery and the Pico's circuitry).
    ///
    /// See the [`led_strip!`](macro@crate::led_strip), [`led_strips!`](crate::led_strips),
    /// and [`led2d!`](mod@crate::led2d) macro docs for usage and context.
    Unlimited,
}

impl Default for Current {
    fn default() -> Self {
        Self::Milliamps(250)
    }
}

// Public so led_strip!/led_strips! expansions in downstream crates can reference it.
#[doc(hidden)]
/// Default electrical current budget for generated LED devices (`Current::Milliamps(250)`).
pub const MAX_CURRENT_DEFAULT: Current = Current::Milliamps(250);

// Public so led_strip!/led_strips! expansions in downstream crates can reference it.
#[doc(hidden)]
/// Default maximum animation frames for generated LED devices (`16`).
pub const MAX_FRAMES_DEFAULT: usize = 16;

impl Current {
    /// Calculate maximum brightness based on electrical current budget and worst-case electrical current draw.
    ///
    /// Returns 255 (full brightness) for Unlimited, or a scaled value for Milliamps.
    #[doc(hidden)] // Called by macro-generated code; not part of public API
    #[must_use]
    pub const fn max_brightness(self, worst_case_ma: u32) -> u8 {
        assert!(worst_case_ma > 0, "worst_case_ma must be positive");
        match self {
            Self::Milliamps(ma) => {
                let scale = (ma as u32 * 255) / worst_case_ma;
                if scale > 255 { 255 } else { scale as u8 }
            }
            Self::Unlimited => 255,
        }
    }
}
