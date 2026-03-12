//! Shared LED-strip building blocks used across all device-envoy platforms.
//!
//! This module provides platform-independent types and traits for NeoPixel-style
//! (WS2812) LED strips. See the platform crate (`device-envoy-rp` or
//! `device-envoy-esp`) for the primary documentation and examples.

// ============================================================================
// Re-exports: color types
// ============================================================================

/// 8-bit RGB color.
///
/// Used in [`Frame1d`] for pixel colors. See [`colors`] for predefined constants.
/// Converts to [`Rgb888`] via [`ToRgb888::to_rgb888`].
#[doc(inline)]
pub use smart_leds::RGB8;

/// Predefined [`RGB8`] color constants (CSS/Web names).
///
/// `GREEN` is `(0, 128, 0)`; `LIME` is `(0, 255, 0)`.
#[doc(inline)]
pub use smart_leds::colors;

/// 8-bit-per-channel RGB color from `embedded-graphics`.
///
/// Get named colors from [`colors`] and convert with [`ToRgb888::to_rgb888`].
/// Converts to [`RGB8`] via [`ToRgb8::to_rgb8`].
#[doc(inline)]
pub use embedded_graphics::pixelcolor::Rgb888;

// ============================================================================
// Color conversion traits
// ============================================================================

/// Convert a color to [`RGB8`] for LED strip rendering.
pub trait ToRgb8 {
    /// Convert to [`RGB8`].
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
        use embedded_graphics::prelude::RgbColor;
        RGB8::new(self.r(), self.g(), self.b())
    }
}

/// Convert a color to [`Rgb888`] for `embedded-graphics` rendering.
pub trait ToRgb888 {
    /// Convert to [`Rgb888`].
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

// ============================================================================
// Gamma correction
// ============================================================================

/// Gamma correction configuration for LED strips.
///
/// See the platform crate's `led_strip!` macro documentation for usage and context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gamma {
    /// No correction; raw LED PWM values.
    Linear,
    /// Perceptual sRGB semantics (gamma 2.2). Preserves named color intent.
    Srgb,
    /// Compatibility with historical `smart_leds::gamma()` curve (2.8).
    SmartLeds,
}

impl Default for Gamma {
    fn default() -> Self {
        Self::Srgb
    }
}

/// Default gamma used by the `led_strip!` macro.
#[doc(hidden)]
pub const GAMMA_DEFAULT: Gamma = Gamma::Srgb;

/// Default max_frames used by the `led_strip!` macro.
#[doc(hidden)]
pub const MAX_FRAMES_DEFAULT: usize = 16;

/// Gamma 2.2 lookup table (sRGB).
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

/// Gamma 2.8 lookup table (matches `smart_leds::gamma()`).
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

const LINEAR_TABLE: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut index = 0usize;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this
    // while loop with a for loop.
    while index < 256 {
        t[index] = index as u8;
        index += 1;
    }
    t
};

/// Build a combined gamma + brightness scaling lookup table (const-evaluable).
///
/// Used by the `led_strip!` macro to generate `COMBO_TABLE` at compile time.
#[doc(hidden)]
#[must_use]
pub const fn generate_combo_table(gamma: Gamma, max_brightness: u8) -> [u8; 256] {
    let gamma_table = match gamma {
        Gamma::Linear => &LINEAR_TABLE,
        Gamma::Srgb => &GAMMA_SRGB_TABLE,
        Gamma::SmartLeds => &GAMMA_SMARTLEDS_TABLE,
    };
    let mut result = [0u8; 256];
    let mut index = 0usize;
    // TODO_NIGHTLY When nightly feature const_for becomes stable, replace this
    // while loop with a for loop.
    while index < 256 {
        let corrected = gamma_table[index];
        result[index] = ((corrected as u16 * max_brightness as u16) / 255) as u8;
        index += 1;
    }
    result
}

// ============================================================================
// Current budget → max brightness
// ============================================================================

/// Current budget for a single LED strip.
///
/// Used by the `led_strip!` macro to derive `MAX_BRIGHTNESS` on the generated struct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Current {
    /// Current limit in milliamps.
    Milliamps(u32),
    /// No limit — full brightness.
    Unlimited,
}

impl Default for Current {
    fn default() -> Self {
        Self::Milliamps(250)
    }
}

impl Current {
    /// Compute the maximum per-channel brightness (0–255) that keeps total
    /// current draw within budget assuming `worst_case_ma` at full brightness.
    ///
    /// Returns 255 (full brightness) for [`Current::Unlimited`], or a scaled value for
    /// [`Current::Milliamps`].
    #[doc(hidden)]
    #[must_use]
    pub const fn max_brightness(self, worst_case_ma: u32) -> u8 {
        assert!(worst_case_ma > 0, "worst_case_ma must be positive");
        match self {
            Self::Milliamps(ma) => {
                let scale = (ma as u64 * 255) / worst_case_ma as u64;
                if scale > 255 { 255 } else { scale as u8 }
            }
            Self::Unlimited => 255,
        }
    }
}

// ============================================================================
// Frame1d
// ============================================================================

use core::ops::{Deref, DerefMut};

/// 1D pixel array used to describe LED strip patterns.
///
/// See the platform crate's `led_strip` module documentation for usage examples.
///
/// Frames deref to `[RGB8; N]`, so you can mutate pixels directly before
/// passing them to the generated strip's `write_frame` method.
#[derive(Clone, Copy, Debug)]
pub struct Frame1d<const N: usize>(pub [RGB8; N]);

impl<const N: usize> Frame1d<N> {
    /// Number of LEDs in this frame.
    pub const LEN: usize = N;

    /// Create a new blank (all-black) frame.
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

// ============================================================================
// Command channel and LedStrip handle
// ============================================================================

use core::borrow::Borrow;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use heapless::Vec;

/// Platform-agnostic LED strip device contract.
///
/// Platform crates implement this for their concrete LED strip types so shared logic can
/// drive strips without knowing the underlying hardware backend.
///
/// This page serves as the definitive reference for what a generated LED strip type
/// provides. For first-time readers, start with the `led_strip` module documentation in your
/// platform crate (`device-envoy-rp` or `device-envoy-esp`), then return here for a
/// complete list of available methods and associated constants.
///
/// Design intent:
///
/// - Primitive operations are [`LedStrip::write_frame`] and [`LedStrip::animate`].
/// - This trait is intended for static dispatch on embedded targets.
///
/// # Example: Write a Single 1-Dimensional Frame
///
/// In this example, we set every other LED to blue and gray.
///
/// ![LED strip preview](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led_strip_simple.png)
///
/// ```rust,no_run
/// use device_envoy_core::led_strip::{Frame1d, LedStrip, colors};
///
/// fn write_alternating_blue_gray<const N: usize>(led_strip: &impl LedStrip<N>) {
///     let mut frame = Frame1d::new();
///     for pixel_index in 0..N {
///         frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
///     }
///     led_strip.write_frame(frame);
/// }
///
/// # struct LedStripSimple;
/// # impl LedStrip<8> for LedStripSimple {
/// #     const MAX_FRAMES: usize = 16;
/// #     const MAX_BRIGHTNESS: u8 = 133;
/// #     fn write_frame(&self, _frame: Frame1d<8>) {}
/// #     fn animate<I>(&self, _frames: I)
/// #     where
/// #         I: IntoIterator,
/// #         I::Item: core::borrow::Borrow<(Frame1d<8>, embassy_time::Duration)>,
/// #     {
/// #     }
/// # }
/// # let led_strip_simple = LedStripSimple;
/// # write_alternating_blue_gray(&led_strip_simple);
/// ```
///
/// # Example: Animate a Sequence
///
/// This example animates a 96-LED strip through red, green, and blue frames, cycling
/// continuously.
///
/// ![LED strip preview](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led_strip_animated.png)
///
/// ```rust,no_run
/// use device_envoy_core::led_strip::{Frame1d, LedStrip, colors};
/// use embassy_time::Duration;
///
/// fn animate_rgb_cycle<const N: usize>(led_strip: &impl LedStrip<N>) {
///     let frame_duration = Duration::from_millis(300);
///     led_strip.animate([
///         (Frame1d::filled(colors::RED), frame_duration),
///         (Frame1d::filled(colors::GREEN), frame_duration),
///         (Frame1d::filled(colors::BLUE), frame_duration),
///     ]);
/// }
///
/// # struct LedStripAnimated;
/// # impl LedStrip<96> for LedStripAnimated {
/// #     const MAX_FRAMES: usize = 3;
/// #     const MAX_BRIGHTNESS: u8 = 44;
/// #     fn write_frame(&self, _frame: Frame1d<96>) {}
/// #     fn animate<I>(&self, _frames: I)
/// #     where
/// #         I: IntoIterator,
/// #         I::Item: core::borrow::Borrow<(Frame1d<96>, embassy_time::Duration)>,
/// #     {
/// #     }
/// # }
/// # let led_strip_animated = LedStripAnimated;
/// # animate_rgb_cycle(&led_strip_animated);
/// ```
pub trait LedStrip<const N: usize> {
    /// Number of LEDs in this strip.
    const LEN: usize = N;
    /// Maximum number of animation frames allowed.
    const MAX_FRAMES: usize;
    /// Maximum brightness level, automatically limited by the power budget.
    const MAX_BRIGHTNESS: u8;

    /// Write a frame to the LED strip.
    ///
    /// See the [LedStrip trait documentation](Self) for usage examples.
    fn write_frame(&self, frame: Frame1d<N>);

    /// Animate frames on the LED strip.
    ///
    /// The duration type is [`embassy_time::Duration`](https://docs.rs/embassy-time/latest/embassy_time/struct.Duration.html), and `frames` can be any iterator whose
    /// items borrow `(Frame1d<N>, embassy_time::Duration)`.
    ///
    /// See the [LedStrip trait documentation](Self) for usage examples.
    fn animate<I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(Frame1d<N>, embassy_time::Duration)>;
}

/// Signal type used to send commands to the background device task.
///
/// `#[doc(hidden)]` because it is named in macro-generated `static` items in
/// downstream crates that call `led_strip!`. Must be `pub` for that use.
#[doc(hidden)]
pub type LedStripCommandSignal<const N: usize, const MAX_FRAMES: usize> =
    Signal<CriticalSectionRawMutex, Command<N, MAX_FRAMES>>;

/// Commands sent from a platform runtime handle to the background device task.
///
/// `#[doc(hidden)]` — implementation detail exposed only for macro expansion.
#[doc(hidden)]
#[derive(Clone)]
pub enum Command<const N: usize, const MAX_FRAMES: usize> {
    /// Display a single static frame indefinitely.
    DisplayStatic(Frame1d<N>),
    /// Loop through a sequence of (frame, duration) pairs.
    Animate(Vec<(Frame1d<N>, Duration), MAX_FRAMES>),
}

// Must be `pub` for macro expansion at foreign call sites — not user-facing.
#[doc(hidden)]
pub fn __write_frame<const N: usize, const MAX_FRAMES: usize>(
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    frame: Frame1d<N>,
) {
    command_signal.signal(Command::DisplayStatic(frame));
}

// Must be `pub` for macro expansion at foreign call sites — not user-facing.
#[doc(hidden)]
pub fn __animate<const N: usize, const MAX_FRAMES: usize, I>(
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
    frames: I,
) where
    I: IntoIterator,
    I::Item: Borrow<(Frame1d<N>, Duration)>,
{
    assert!(MAX_FRAMES > 0, "animation disabled (MAX_FRAMES = 0)");
    let mut sequence: Vec<(Frame1d<N>, Duration), MAX_FRAMES> = Vec::new();
    for item in frames {
        let (frame, duration) = *item.borrow();
        assert!(
            duration.as_micros() > 0,
            "animation frame duration must be positive"
        );
        sequence
            .push((frame, duration))
            .expect("animation sequence fits within MAX_FRAMES");
    }
    assert!(
        !sequence.is_empty(),
        "animation requires at least one frame"
    );
    command_signal.signal(Command::Animate(sequence));
}

/// Static resources for a LED strip runtime instance. Allocated once at program
/// start (typically as a `static`).
///
/// `#[doc(hidden)]` — exposed only for macro expansion in downstream crates.
#[doc(hidden)]
pub struct LedStripStatic<const N: usize, const MAX_FRAMES: usize> {
    command_signal: LedStripCommandSignal<N, MAX_FRAMES>,
}

impl<const N: usize, const MAX_FRAMES: usize> LedStripStatic<N, MAX_FRAMES> {
    /// Create the static resources. Call from a `static` initializer.
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

// ============================================================================
// apply_correction
// ============================================================================

/// Apply the combo (gamma + brightness) table to every pixel in a frame.
///
/// `#[doc(hidden)]` — called from platform-specific device loops.
#[doc(hidden)]
pub fn apply_correction<const N: usize>(frame: &mut Frame1d<N>, combo_table: &[u8; 256]) {
    frame.iter_mut().for_each(|pixel| {
        pixel.r = combo_table[pixel.r as usize];
        pixel.g = combo_table[pixel.g as usize];
        pixel.b = combo_table[pixel.b as usize];
    });
}
