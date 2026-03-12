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
//! #     match example(spawner).await {
//! #         Ok(infallible) => match infallible {},
//! #         Err(error) => panic!("{error:?}"),
//! #     }
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
//! #     match example(spawner).await {
//! #         Ok(infallible) => match infallible {},
//! #         Err(error) => panic!("{error:?}"),
//! #     }
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
    bit_matrix3x4_font, render_text_to_frame, Frame2d, Led2dFont, Led2dStripAdapter,
    Led2dStripBacked, LedLayout, Point, Size,
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
/// - `engine` - Transport engine (`Engine::Rmt` or `Engine::Spi`, default: `Engine::Rmt`).
/// - `gamma` - Color correction curve (default: `Gamma::Srgb`).
/// - `max_frames` - Maximum number of animation frames (default: 16).
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led_strip!`](mod@crate::led_strip) - For 1-dimensional LED strips.
#[doc(hidden)]
#[macro_export]
macro_rules! led2d {
    ($($tt:tt)*) => { $crate::__led2d_impl! { $($tt)* } };
}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use led2d;

#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_impl {
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led2d_impl! {
            $name,
            pin = [],
            len = [],
            led_layout = [],
            max_current = [],
            font = [],
            engine = [],
            gamma = [],
            max_frames = [],
            fields = [$($fields)*],
        }
    };

    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [$led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [$font:expr],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        $crate::__led2d_dispatch_engine!(
            $name,
            $pin,
            $len,
            $led_layout,
            $crate::__led_strip_max_current_or_default!([$($max_current)?]),
            $font,
            [$($engine)?],
            [$($gamma)?],
            [$($max_frames)?],
        );
    };
    (
        $name:ident,
        pin = [],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `pin` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `len` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `led_layout` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [$led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `font` field");
    };
    (
        $name:ident,
        pin = [],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$pin],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$already_pin:ident],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `pin` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$len],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$already_len:expr],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `len` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [led_layout: $led_layout:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$led_layout],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$already_led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [led_layout: $led_layout:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `led_layout` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$max_current],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$already_max_current:expr],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `max_current` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [font: $font:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$font],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$already_font:expr],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [font: $font:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `font` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $crate::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $crate::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$already_engine:tt],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `engine` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [],
        max_frames = [$($max_frames:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$gamma],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$already_gamma:expr],
        max_frames = [$($max_frames:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `gamma` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_impl!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$max_frames],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$already_max_frames:expr],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `max_frames` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [$field:ident : $value:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!(
            "led2d! unknown field; expected `pin`, `len`, `led_layout`, `font`, `max_current`, `engine`, `gamma`, or `max_frames`"
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_dispatch_engine {
    (
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
        $crate::led_strip::__led_strip_inner!{
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
        $crate::led_strip::__led_strip_inner!{
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
}
