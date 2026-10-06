#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("led2d1", "docs/assets/led2d1.png"),
    doc = ::embed_doc_image::embed_image!("led2d2", "docs/assets/led2d2.png")
)]
//! A device abstraction for rectangular NeoPixel-style (WS2812) LED panel displays.
//! For 1-dimensional LED strips, see the [`led_strip`](mod@crate::led_strip) module.
//!
//! This page provides the primary documentation and examples for programming LED panels.
//! The device abstraction supports text, graphics, and animation.
//!
//! **After reading the examples below, see also:**
//!
//! - [`led2d!`](macro@crate::led2d) - Macro to generate an LED-panel struct type (includes syntax details).
//! - [`Led2d`](`crate::led2d::Led2d`) - Core trait that defines the LED panel API surface.
//! - [`Led2dGenerated`](led2d_generated::Led2dGenerated) - Sample generated panel type showing the constructor path.
//! - [`LedLayout`] - Compile-time description of panel geometry and wiring, including dimensions (with examples)
//! - [`Frame2d`] - 2D pixel array used for general graphics (includes examples)
//! - [`led_strip!`](mod@crate::led_strip) - Underlying strip abstraction used by this panel API.
//!
//! # Example: Write Text
//!
//! In this example, we render text on a 12x4 panel. Here, the generated struct type is named `Led12x4`.
//!
//! ![LED panel preview][led2d1]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::{
//!     Result, init_and_start, led2d,
//!     led2d::{Led2d as _, Led2dFont, layout::LedLayout},
//!     led_strip::colors,
//! };
//!
//! // Tells us how the LED strip is wired up in the panel.
//! // In this case, a common snake-like pattern.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//!
//! // Generate a type named `Led12x4`.
//! led2d! {
//!     Led12x4 {
//!         pin: GPIO18,                       // GPIO pin for LED data signal
//!         len: 48,                           // Number of LEDs in the panel
//!         led_layout: LED_LAYOUT_12X4,       // LED layout mapping (defines dimensions)
//!         font: Led2dFont::Font3x4Trim,      // Font variant
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
//!
//!     // Create a device abstraction for the LED panel.
//!     // Behind the scenes, this creates a channel and background task to manage the display.
//!     let led12x4 = Led12x4::new(p.GPIO18, rmt80.channel0, spawner)?;
//!
//!     // Write text to the display with per-character colors.
//!     let colors = [colors::CYAN, colors::RED, colors::YELLOW];
//!     // Each character takes the next color; when we run out, we start over.
//!     led12x4.write_text("Rust", &colors);
//!
//!     core::future::pending().await
//! }
//! ```
//!
//! # Example: Animated Text on a Rotated Panel
//!
//! This example animates text on a rotated 12x8 panel built from two stacked 12x4 panels.
//!
//! ![LED panel preview][led2d2]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::{
//!     Result, init_and_start, led2d,
//!     led2d::{Frame2d, Led2d as _, Led2dFont, layout::LedLayout},
//!     led_strip::{Current, Gamma, colors},
//! };
//! use embassy_time::Duration;
//!
//! // Our panel is two 12x4 panels stacked vertically and then rotated clockwise.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//! const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
//! const LED_LAYOUT_8X12_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
//!
//! // Generate a type named `Led12x8Animated`.
//! led2d! {
//!     Led12x8Animated {
//!         pin: GPIO18,                           // GPIO pin for LED data signal
//!         len: 96,                               // Number of LEDs in the panel
//!         led_layout: LED_LAYOUT_8X12_ROTATED,  // Two 12x4 panels stacked and rotated
//!         max_current: Current::Milliamps(300), // Power budget, default is 250 mA
//!         font: Led2dFont::Font4x6Trim,         // 4x6 font without normal padding
//!         gamma: Gamma::Linear,                 // Color correction curve, default is Gamma::Srgb
//!         max_frames: 2,                        // Maximum animation frames, default is 16
//!     }
//! }
//!
//! # #[esp_rtos::main]
//! # async fn main(spawner: embassy_executor::Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
//!
//!     // Create a device abstraction for the rotated LED panel.
//!     let led12x8_animated = Led12x8Animated::new(p.GPIO18, rmt80.channel0, spawner)?;
//!
//!     // Write "Go" into an in-memory frame buffer.
//!     let mut frame_0 = Frame2d::new();
//!     // Empty text colors array defaults to white.
//!     led12x8_animated.write_text_to_frame("Go", &[], &mut frame_0);
//!
//!     // Write "Go" into a second frame buffer with custom colors and on the 2nd line.
//!     let mut frame_1 = Frame2d::new();
//!     // "\n" starts a new line. Text does not wrap but rather clips.
//!     led12x8_animated.write_text_to_frame("\nGo", &[colors::HOT_PINK, colors::LIME], &mut frame_1);
//!
//!     // Animate between the two frames indefinitely.
//!     let frame_duration = Duration::from_secs(1);
//!     led12x8_animated.animate([(frame_0, frame_duration), (frame_1, frame_duration)]);
//!
//!     core::future::pending().await
//! }
//! ```
//!
pub mod layout {
    pub use device_envoy_core::led2d::layout::*;
}

pub use device_envoy_core::led2d::Led2d;
pub use device_envoy_core::led2d::{
    Frame2d, Led2dFont, Led2dStripAdapter, Led2dStripBacked, LedLayout, Point, Size,
    bit_matrix3x4_font, render_text_to_frame,
};
pub mod led2d_generated;

// Must be `pub` (not `pub(crate)`) because called by macro-generated code that
// expands at the call site in downstream crates.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
pub type Led2dEsp<'a, const N: usize, S> = Led2dStripAdapter<'a, N, S>;

/// Macro to generate an LED-panel struct type (includes syntax details). See [`Led2d`](`crate::led2d::Led2d`) for the shared API.
///
/// **See the [led2d module](mod@crate::led2d) for usage examples.**
///
/// **Syntax:**
///
/// ```text
/// led2d! {
///     <Name> {
///         pin: <pin_ident>,
///         len: <usize_expr>,
///         led_layout: <LedLayout_expr>,
///         font: <Led2dFont_expr>,
///         max_current: <Current_expr>, // optional
///         engine: Engine::Rmt|Engine::Spi, // optional
///         gamma: <Gamma_expr>, // optional
///         max_frames: <usize_expr>, // optional
///     }
/// }
/// ```
///
/// # Fields
///
/// **Required fields:**
///
/// - `pin` - GPIO pin for LED data.
/// - `len` - Number of LEDs in the generated strip.
/// - `led_layout` - LED strip physical layout (see [`LedLayout`]); this defines panel size.
/// - `font` - Built-in font variant (see [`Led2dFont`]), for example `Led2dFont::Font4x6Trim`.
///
/// The `led_layout` value must be a const so its dimensions can be derived at compile time.
///
/// **Optional fields:**
///
/// - `max_current` - Electrical current budget (default: 250 mA).
/// - `engine` - Transport engine (`Engine::Rmt` or `Engine::Spi`, default: RMT on RMT-capable chips, otherwise SPI).
/// - `gamma` - Color correction curve (default: `Gamma::Srgb`).
/// - `max_frames` - Maximum number of animation frames (default: 16).
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
/// Code generator for [`led2d!`](crate::led2d::led2d).
///
/// Called only by `led2d!` after its `const_structures::define!` schema has validated the
/// input and filled defaults. The engine backends apply the user's name, visibility,
/// attributes, and generated docs directly to the panel struct. Must be
/// public for macro expansion in downstream crates, but not user-facing API.
#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        len: $len:expr,
        led_layout: $led_layout:expr,
        font: $font:expr,
        max_current: $max_current:expr,
        engine: [$($engine:tt)*],
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
    ) => {
        $crate::__led_engine_normalize! {
            panel,
            [$($engine)*],
            { [$(#[$attr])* #[doc = $doc]], [$vis], $name, $pin, $len, $led_layout, $max_current, $font, },
            { [$gamma], [$max_frames], }
        }
    };
}

const_structures::define! {
    ///
    /// # Related Macros
    ///
    /// - [`led_strip!`](mod@crate::led_strip) - For 1-dimensional LED strips.
    #[cfg(target_os = "none")]
    pub led2d => __led2d_generate {
        /// GPIO pin for LED data, for example `GPIO8`.
        pin: ident,
        /// Number of LEDs (pixels); must match `led_layout`.
        len: expr,
        /// Physical layout; a `const` `LedLayout` that defines the panel size.
        led_layout: expr,
        /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
        font: expr,
        /// Electrical current budget.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = $crate::led_strip::CURRENT_DEFAULT,
        /// Output engine, `Engine::Rmt` or `Engine::Spi`; defaults to RMT on RMT-capable chips, otherwise SPI.
        engine?: expr,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = $crate::led_strip::GAMMA_DEFAULT,
        /// Maximum number of animation frames; `0` disables animation.
        #[default_display = "16"]
        max_frames: expr = $crate::led_strip::MAX_FRAMES_DEFAULT,
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_dispatch_engine {
    (
        [$($attrs:tt)*],
        [$vis:vis],
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [Spi],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
    ) => {
        $crate::led_strip::spi::__led_strip_spi_inner!{
            [$($attrs)*],
            [$vis],
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [$led_layout],
            [$font],
        }
    };
    (
        [$($attrs:tt)*],
        [$vis:vis],
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [Rmt],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
    ) => {
        $crate::__led_strip_dispatch_rmt_engine!{
            [$($attrs)*],
            [$vis],
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        [$($attrs:tt)*],
        [$vis:vis],
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
    ) => {
        $crate::__led_strip_dispatch_default_engine!{
            [$($attrs)*],
            [$vis],
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        [$($attrs:tt)*],
        [$vis:vis],
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [$($engine:tt)*],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
    ) => {
        compile_error!("led2d! `engine` must be `Engine::Rmt` or `Engine::Spi`");
    };
}
