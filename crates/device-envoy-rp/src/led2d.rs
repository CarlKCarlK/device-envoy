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
//! - [`led2d!`](macro@crate::led2d) — Macro to generate an LED-panel struct type (includes syntax details).
//! - [`Led2d`](`crate::led2d::Led2d`) — Core trait that defines the LED panel API surface.
//! - [`Led2dGenerated`](led2d_generated::Led2dGenerated) — Sample generated panel type showing the constructor path.
//! - [`LedLayout`] — Compile-time description of panel geometry and wiring, including dimensions (with examples)
//! - [`Frame2d`] — 2D pixel array used for general graphics (includes examples)
//! - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other panels or LED strips (includes examples)
//!
//! # Example: Write Text
//!
//! In this example, we render text on a 12×4 panel. Here, the generated struct type is named `Led12x4`.
//!
//! ![LED panel preview][led2d1]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use core::result::Result::Ok;
//! # use embassy_executor::Spawner;
//! # use embassy_rp::init;
//! use device_envoy_rp::{Result, led2d, led2d::layout::LedLayout, led2d::Led2dFont, led_strip::colors, led2d::Led2d as _};
//!
//! // Tells us how the LED strip is wired up in the panel
//! // in this case, a common snake-like pattern.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//!
//! // Generate a type named `Led12x4`.
//! led2d! {
//!     Led12x4 {
//!         pin: PIN_3,                          // GPIO pin for LED data signal
//!         led_layout: LED_LAYOUT_12X4,         // LED layout mapping (defines dimensions)
//!         font: Led2dFont::Font3x4Trim,        // Font variant
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # pub async fn main(spawner: Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: Spawner) -> Result<Infallible> {
//!     let p = init(Default::default());
//!
//!     // Create a device abstraction for the LED panel.
//!     // Behind the scenes, this creates a channel & background task to manage the display.
//!     let led12x4 = Led12x4::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     // Write text to the display with per-character colors.
//!     let colors = [colors::CYAN, colors::RED, colors::YELLOW];
//!     // Each character takes the next color; when we run out, we start over.
//!     led12x4.write_text("Rust", &colors);
//!
//!     pending().await // run forever
//! }
//! ```
//!
//! # Example: Animated Text on a Rotated Panel
//!
//! This example animates text on a rotated 12×8 panel built from two stacked 12×4 panels.
//!
//! ![LED panel preview][led2d2]
//!
//! ```rust,no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::{convert::Infallible, future::pending};
//! # use embassy_executor::Spawner;
//! # use embassy_rp::init;
//! use device_envoy_rp::{Result, led2d, led2d::layout::LedLayout, led2d::Frame2d, led2d::Led2dFont, led_strip::{Current, Gamma, colors}, led2d::Led2d as _};
//! use embassy_time::Duration;
//!
//! // Our panel is two 12x4 panels stacked vertically and then rotated clockwise.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//! const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.combine_v(LED_LAYOUT_12X4);
//! const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
//!
//! // Generate a type named `Led12x8Animated`.
//! led2d! {
//!     pub(self) Led12x8Animated {               // Can provide a visibility modifier
//!         pin: PIN_4,                           // GPIO pin for LED data signal
//!         led_layout: LED_LAYOUT_12X8_ROTATED,  // Two 12×4 panels stacked and rotated
//!         font: Led2dFont::Font4x6Trim,         // Use a 4x6 pixel font without the usual 1 pixel padding
//!         pio: PIO1,                            // PIO resource, default is PIO0
//!         dma: DMA_CH1,                         // DMA resource, default is DMA_CH0
//!         max_current: Current::Milliamps(300), // Power budget, default is 250 mA.
//!         gamma: Gamma::Linear,                 // Color correction curve, default is Gamma::Srgb
//!         max_frames: 2,                        // maximum animation frames, default is 16
//!     }
//! }
//!
//! # #[embassy_executor::main]
//! # pub async fn main(spawner: Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: Spawner) -> Result<Infallible> {
//!     let p = init(Default::default());
//!
//!    // Create a device abstraction for the rotated LED panel.
//!     let led_12x8_animated = Led12x8Animated::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;
//!
//!     // Write "Go" into an in-memory frame buffer.
//!     let mut frame_0 = Frame2d::new();
//!     // Empty text colors array defaults to white.
//!     led_12x8_animated.write_text_to_frame("Go", &[], &mut frame_0);
//!
//!     // Write "Go" into a second frame buffer with custom colors and on the 2nd line.
//!     let mut frame_1 = Frame2d::new();
//!     // "\n" starts a new line. Text does not wrap but rather clips.
//!     led_12x8_animated.write_text_to_frame(
//!         "\nGo",
//!         &[colors::HOT_PINK, colors::LIME],
//!         &mut frame_1,
//!     );
//!
//!     // Animate between the two frames indefinitely.
//!     let frame_duration = Duration::from_secs(1);
//!     led_12x8_animated
//!         .animate([(frame_0, frame_duration), (frame_1, frame_duration)]);
//!
//!     pending().await // run forever
//! }
//! ```

// Re-export for macro use
#[doc(hidden)]
pub use paste;

/// Layout mapping types for 2D LED panels. See the [`led2d` module documentation](mod@crate::led2d) for details.
pub mod layout {
    pub use device_envoy_core::led2d::layout::*;
}

pub use device_envoy_core::led2d::Led2d;
pub use device_envoy_core::led2d::{
    Frame2d, Led2dFont, Led2dStripAdapter, Led2dStripBacked, Point, Size, bit_matrix3x4_font,
    render_text_to_frame,
};
pub use layout::LedLayout;
pub mod led2d_generated;

// Must be `pub` (not `pub(crate)`) because called by macro-generated code that expands at the call site in downstream crates.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
pub type Led2dRp<'a, const N: usize, S> = Led2dStripAdapter<'a, N, S>;

/// Code generator for [`led2d!`](crate::led2d::led2d): a one-member
/// [`led_strips!`](crate::led_strip::led_strips) group whose member is a 2D panel, plus a
/// constructor that hides the group.
///
/// Called only by `led2d!` after its `const_structures::define!` schema has validated the input and
/// filled defaults. Must be public for macro expansion in downstream crates, but not
/// user-facing API.
#[cfg(not(feature = "host"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        led_layout: $led_layout:expr,
        font: $font:expr,
        pio: $pio:ident,
        dma: $dma:ident,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
    ) => {
        $crate::__paste! {
            $crate::__led_strips_generate! {
                attrs: [],
                vis: [pub(self)],
                name: [<$name Group>],
                doc: "One-member group behind a `led2d!` type.",
                pio: $pio,
                member_count: 1,
                members: [{
                    index: 0,
                    attrs: [$(#[$attr])*],
                    vis: [$vis],
                    name: $name,
                    doc: $doc,
                    pin: $pin,
                    len: $led_layout.len(),
                    max_current: $max_current,
                    dma: $dma,
                    gamma: $gamma,
                    max_frames: $max_frames,
                    led2d: [{ led_layout: $led_layout, font: $font, }],
                },],
            }

            impl $name {
                /// Creates the LED panel and spawns its background task.
                ///
                /// The `pin`, `pio`, and `dma` arguments must be the GPIO pin, PIO resource,
                /// and DMA channel named in the macro. See the
                /// [led2d module documentation](mod@device_envoy_rp::led2d) for usage.
                pub fn new(
                    pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>>,
                    pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>>,
                    dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<Self> {
                    let (led2d,) = [<$name Group>]::new(pio, pin, dma, spawner)?;
                    Ok(led2d)
                }
            }
        }
    };
}

// Internal macro used by led_strips! led2d configuration.
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! led2d_from_strip {
    // Serpentine column-major led_layout variant (uses strip's MAX_FRAMES)
    (
        $vis:vis $name:ident,
        strip_type: $strip_type:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: serpentine_column_major,
        font: $font_variant:expr $(,)?
    ) => {
        $crate::led2d::paste::paste! {
            const [<$name:upper _LED_LAYOUT>]: $crate::led2d::LedLayout<{ $width * $height }, { $width }, { $height }> =
                $crate::led2d::LedLayout::<{ $width * $height }, { $width }, { $height }>::serpentine_column_major();
            const [<$name:upper _MAX_FRAMES>]: usize = $strip_type::MAX_FRAMES;

            // Compile-time assertion that strip length matches led_layout length
            const _: () = assert!([<$name:upper _LED_LAYOUT>].index_to_xy().len() == $strip_type::LEN);

            $crate::led2d::led2d_from_strip!(
                @common $vis, $name, $strip_type, [<$name:upper _LED_LAYOUT>],
                $font_variant,
                [<$name:upper _MAX_FRAMES>]
            );
        }
    };
    // Custom led_layout variant (uses strip's MAX_FRAMES)
    (
        $(#[$attr:meta])*
        $vis:vis $name:ident,
        strip_type: $strip_type:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: $led_layout:expr,
        font: $font_variant:expr $(,)?
    ) => {
        $crate::led2d::paste::paste! {
            const [<$name:upper _LED_LAYOUT>]: $crate::led2d::LedLayout<{ $width * $height }, { $width }, { $height }> = $led_layout;
            const [<$name:upper _MAX_FRAMES>]: usize = $strip_type::MAX_FRAMES;

            // Compile-time assertion that strip length matches led_layout length
            const _: () = assert!([<$name:upper _LED_LAYOUT>].index_to_xy().len() == $strip_type::LEN);

            $crate::led2d::led2d_from_strip!(
                @common $(#[$attr])* $vis, $name, $strip_type, [<$name:upper _LED_LAYOUT>],
                $font_variant,
                [<$name:upper _MAX_FRAMES>]
            );
        }
    };
    // Internal: use existing led_layout const (avoids redundant constants)
    (
        @__from_layout_const
        $vis:vis $name:ident,
        strip_type: $strip_type:ident,
        led_layout_const: $led_layout_const:ident,
        font: $font_variant:expr,
        max_frames_const: $max_frames_const:ident $(,)?
    ) => {
        $crate::led2d::led2d_from_strip!(
            @common $vis, $name, $strip_type, $led_layout_const,
            $font_variant,
            $max_frames_const
        );
    };
    // Common implementation (shared by both variants)
    (
        @common $(#[$attr:meta])* $vis:vis,
        $name:ident,
        $strip_type:ident,
        $led_layout_const:ident,
        $font_variant:expr,
        $max_frames_const:ident
    ) => {
        $crate::led2d::paste::paste! {
            $(#[$attr])*
            /// LED matrix device handle generated by [`led2d_from_strip!`](crate::led2d::led2d_from_strip).
            $vis struct [<$name>] {
                led2d: $crate::led2d::Led2dRp<'static, { $led_layout_const.len() }, $strip_type>,
            }

            #[allow(non_snake_case, dead_code)]
            impl [<$name>] {
                /// Maximum number of animation frames.
                pub const MAX_FRAMES: usize = $max_frames_const;
                /// Maximum brightness level after current limiting.
                pub const MAX_BRIGHTNESS: u8 = $strip_type::MAX_BRIGHTNESS;
                /// Default font used by text helpers.
                pub const FONT: $crate::led2d::Led2dFont = $font_variant;
                /// Panel width in pixels.
                pub const WIDTH: usize = $led_layout_const.width();
                /// Panel height in pixels.
                pub const HEIGHT: usize = $led_layout_const.height();
                /// Total LED count (`WIDTH * HEIGHT`).
                pub const LEN: usize = $led_layout_const.len();
                /// Panel dimensions as a [`Size`](crate::led2d::Size).
                pub const SIZE: $crate::led2d::Size =
                    $crate::led2d::Frame2d::<{ $led_layout_const.width() }, { $led_layout_const.height() }>::SIZE;
                /// Top-left corner coordinate.
                pub const TOP_LEFT: $crate::led2d::Point =
                    $crate::led2d::Frame2d::<{ $led_layout_const.width() }, { $led_layout_const.height() }>::TOP_LEFT;
                /// Top-right corner coordinate.
                pub const TOP_RIGHT: $crate::led2d::Point =
                    $crate::led2d::Frame2d::<{ $led_layout_const.width() }, { $led_layout_const.height() }>::TOP_RIGHT;
                /// Bottom-left corner coordinate.
                pub const BOTTOM_LEFT: $crate::led2d::Point =
                    $crate::led2d::Frame2d::<{ $led_layout_const.width() }, { $led_layout_const.height() }>::BOTTOM_LEFT;
                /// Bottom-right corner coordinate.
                pub const BOTTOM_RIGHT: $crate::led2d::Point =
                    $crate::led2d::Frame2d::<{ $led_layout_const.width() }, { $led_layout_const.height() }>::BOTTOM_RIGHT;

                // Public so led2d_from_strip! expansions in downstream crates can call it.
                #[doc(hidden)]
                $vis fn from_strip(
                    led_strip: &'static $strip_type,
                ) -> $crate::Result<Self> {
                    let led2d = $crate::led2d::Led2dRp::new(
                        led_strip,
                        &$led_layout_const,
                    );

                    defmt::info!("Led2dRp::new: device created successfully");
                    Ok(Self { led2d })
                }
            }

            impl $crate::led2d::Led2d<{ $led_layout_const.width() }, { $led_layout_const.height() }>
                for [<$name>]
            {
                const MAX_FRAMES: usize = $max_frames_const;
                const MAX_BRIGHTNESS: u8 = $strip_type::MAX_BRIGHTNESS;
                const FONT: $crate::led2d::Led2dFont = $font_variant;

                fn write_frame(
                    &self,
                    frame2d: $crate::led2d::Frame2d<{ $led_layout_const.width() }, { $led_layout_const.height() }>,
                ) {
                    $crate::led2d::Led2dStripBacked::write_frame(&self.led2d, frame2d);
                }

                fn animate<I>(&self, frames: I)
                where
                    I: IntoIterator,
                    I::Item: core::borrow::Borrow<(
                        $crate::led2d::Frame2d<{ $led_layout_const.width() }, { $led_layout_const.height() }>,
                        embassy_time::Duration,
                    )>,
                {
                    $crate::led2d::Led2dStripBacked::animate(&self.led2d, frames);
                }
            }
        }
    };
}

const_structures::define! {
    /// Macro to generate an LED-panel struct type. See [`Led2d`](`crate::led2d::Led2d`) for the shared API.
    ///
    /// **See the [led2d module](mod@crate::led2d) for usage examples.**
    ///
    /// The `led_layout` value must be a const so its dimensions can be derived at compile time.
    ///
    /// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
    ///
    #[doc = include_str!("docs/current_limiting_and_gamma.md")]
    ///
    /// # Related Macros
    ///
    /// - [`led_strips!`](crate::led_strips) — Alternative macro to share a PIO resource with other panels or LED strips (includes examples)
    /// - [`led_strip!`](mod@crate::led_strip) — For 1-dimensional LED strips
    #[cfg(not(feature = "host"))]
    pub led2d => __led2d_generate {
        /// GPIO pin for LED data, for example `PIN_4`.
        pin: ident,
        /// Physical layout; a `const` `LedLayout` that defines the panel size.
        led_layout: expr,
        /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
        font: expr,
        /// PIO resource.
        pio: ident = PIO0,
        /// DMA channel.
        dma: ident = DMA_CH0,
        /// Electrical current budget.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = $crate::led_strip::MAX_CURRENT_DEFAULT,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = $crate::led_strip::Gamma::Srgb,
        /// Maximum number of animation frames; `0` disables animation.
        max_frames: expr = 16,
    }
}
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub use led2d_from_strip;
