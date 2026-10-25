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
//! - [`Led2d`] - Core trait that defines the LED panel API surface.
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
//!     init_and_start!(p);
//!     let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
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
//!     init_and_start!(p);
//!     let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
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

macro_schema::define! {
    /// Macro to generate an LED-panel struct type. See [`Led2d`](crate::led2d::Led2d) for the shared API.
    ///
    /// **See the [led2d module](mod@crate::led2d) for usage examples.**
    ///
    /// # Fields
    ///
    /// The `led_layout` value must be a const so its dimensions can be derived at compile time.
    ///
    /// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
    ///
    #[doc = include_str!("docs/current_limiting_and_gamma.md")]
    ///
    /// # Related Macros
    ///
    /// - [`led_strip!`](mod@crate::led_strip) - For 1-dimensional LED strips.
    #[cfg(target_os = "none")]
    pub led2d {
        /// GPIO pin for LED data, for example `GPIO8`.
        pin: ident,
        /// Physical layout; a `const` `LedLayout` that defines the panel size and its number of LEDs.
        led_layout: expr,
        /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
        font: expr,
        /// Electrical current budget for this device; brightness is scaled to stay within it.
        /// Budgets are per device: several separately declared devices on one supply each get
        /// the default, so set it explicitly when their total matters.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = $crate::led_strip::CURRENT_DEFAULT,
        /// Output engine, `Rmt` or `Spi`.
        #[default_display = "RMT where the chip has it, otherwise SPI"]
        engine: ty = $crate::led_strip::DefaultEngine,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = $crate::led_strip::GAMMA_DEFAULT,
        /// Maximum number of animation frames; `0` disables animation.
        #[default_display = "16"]
        max_frames: expr = $crate::led_strip::MAX_FRAMES_DEFAULT,
    }

    generate {
        $crate::led_strip::__led_strip_type! {
            #[doc = "LED strip behind a 2D panel generated by `led2d!`."]
            $decl.vis $ident($decl.name, LedStrip) {
                pin: $decl.pin,
                len: $decl.led_layout.len(),
                max_current: $decl.max_current,
                engine: $decl.engine,
                gamma: $decl.gamma,
                max_frames: $decl.max_frames,
            }
        }

        const $upper($decl.name, _LED_LAYOUT): $crate::led2d::LedLayout<
            { $decl.led_layout.len() },
            { $decl.led_layout.width() },
            { $decl.led_layout.height() },
        > = $decl.led_layout;

        // The panel's layout must cover exactly the strip's LEDs.
        const _: () = assert!(
            $upper($decl.name, _LED_LAYOUT).index_to_xy().len() == $ident($decl.name, LedStrip)::LEN
        );

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            led2d: $crate::led2d::Led2dEsp<
                'static,
                { $upper($decl.name, _LED_LAYOUT).len() },
                $ident($decl.name, LedStrip),
            >,
            strip: &'static $ident($decl.name, LedStrip),
        }

        impl $decl.name {
            /// Maximum number of animation frames.
            pub const MAX_FRAMES: usize = $ident($decl.name, LedStrip)::MAX_FRAMES;
            /// Maximum brightness level after current limiting.
            pub const MAX_BRIGHTNESS: u8 = $ident($decl.name, LedStrip)::MAX_BRIGHTNESS;
            /// Default font used by text helpers.
            pub const FONT: $crate::led2d::Led2dFont = $decl.font;
            /// Panel width in pixels.
            pub const WIDTH: usize = $upper($decl.name, _LED_LAYOUT).width();
            /// Panel height in pixels.
            pub const HEIGHT: usize = $upper($decl.name, _LED_LAYOUT).height();
            /// Total LED count (`WIDTH * HEIGHT`).
            pub const LEN: usize = $upper($decl.name, _LED_LAYOUT).len();
            /// Panel dimensions as a [`Size`](device_envoy_rp::led2d::Size).
            pub const SIZE: $crate::led2d::Size = $crate::led2d::Frame2d::<
                { $upper($decl.name, _LED_LAYOUT).width() },
                { $upper($decl.name, _LED_LAYOUT).height() },
            >::SIZE;
            /// Top-left corner coordinate.
            pub const TOP_LEFT: $crate::led2d::Point = $crate::led2d::Frame2d::<
                { $upper($decl.name, _LED_LAYOUT).width() },
                { $upper($decl.name, _LED_LAYOUT).height() },
            >::TOP_LEFT;
            /// Top-right corner coordinate.
            pub const TOP_RIGHT: $crate::led2d::Point = $crate::led2d::Frame2d::<
                { $upper($decl.name, _LED_LAYOUT).width() },
                { $upper($decl.name, _LED_LAYOUT).height() },
            >::TOP_RIGHT;
            /// Bottom-left corner coordinate.
            pub const BOTTOM_LEFT: $crate::led2d::Point = $crate::led2d::Frame2d::<
                { $upper($decl.name, _LED_LAYOUT).width() },
                { $upper($decl.name, _LED_LAYOUT).height() },
            >::BOTTOM_LEFT;
            /// Bottom-right corner coordinate.
            pub const BOTTOM_RIGHT: $crate::led2d::Point = $crate::led2d::Frame2d::<
                { $upper($decl.name, _LED_LAYOUT).width() },
                { $upper($decl.name, _LED_LAYOUT).height() },
            >::BOTTOM_RIGHT;

            /// The LED strip behind this panel, for writing pixels in wiring order.
            pub fn strip(&self) -> &'static $ident($decl.name, LedStrip) {
                self.strip
            }

            /// Creates the panel and spawns its background task. `resource` is the engine's
            /// peripheral: an RMT channel creator for `Rmt`, or an SPI instance for `Spi`.
            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                resource: impl $crate::led_strip::EngineResource<$ident($decl.name, LedStripEngine)>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                let led_strip = $ident($decl.name, LedStrip)::new(pin, resource, spawner)?;
                static PANEL_CELL: $crate::__static_cell::StaticCell<$decl.name> =
                    $crate::__static_cell::StaticCell::new();
                Ok(PANEL_CELL.init(Self {
                    led2d: $crate::led2d::Led2dEsp::new(led_strip, &$upper($decl.name, _LED_LAYOUT)),
                    strip: led_strip,
                }))
            }
        }

        impl $crate::led2d::Led2d<
            { $upper($decl.name, _LED_LAYOUT).width() },
            { $upper($decl.name, _LED_LAYOUT).height() },
        > for $decl.name {
            const MAX_FRAMES: usize = $ident($decl.name, LedStrip)::MAX_FRAMES;
            const MAX_BRIGHTNESS: u8 = $ident($decl.name, LedStrip)::MAX_BRIGHTNESS;
            const FONT: $crate::led2d::Led2dFont = $decl.font;

            fn write_frame(
                &self,
                frame2d: $crate::led2d::Frame2d<
                    { $upper($decl.name, _LED_LAYOUT).width() },
                    { $upper($decl.name, _LED_LAYOUT).height() },
                >,
            ) {
                $crate::led2d::Led2dStripBacked::write_frame(&self.led2d, frame2d);
            }

            fn animate<I>(&self, frames: I)
            where
                I: IntoIterator,
                I::Item: ::core::borrow::Borrow<(
                    $crate::led2d::Frame2d<
                        { $upper($decl.name, _LED_LAYOUT).width() },
                        { $upper($decl.name, _LED_LAYOUT).height() },
                    >,
                    ::embassy_time::Duration,
                )>,
            {
                $crate::led2d::Led2dStripBacked::animate(&self.led2d, frames);
            }
        }
    }
}
