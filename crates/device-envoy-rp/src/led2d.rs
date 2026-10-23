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
//! - [`Led2d`] — Core trait that defines the LED panel API surface.
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

macro_schema::define! {
    /// Macro to generate an LED-panel struct type. See [`Led2d`](crate::led2d::Led2d) for the shared API.
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
    pub led2d {
        /// GPIO pin for LED data, for example `PIN_4`.
        pin: ident,
        /// Physical layout; a `const` `LedLayout` that defines the panel size.
        led_layout: expr,
        /// Built-in font for text, for example `Led2dFont::Font4x6Trim`.
        font: expr,
        /// PIO resource.
        /// Each PIO serves one device or group: if two use the same PIO, the program doesn't
        /// compile (typically `use of moved value` at the second constructor), so give one of
        /// them another PIO.
        pio: ident = PIO0,
        /// DMA channel.
        dma: ident = DMA_CH0,
        /// Electrical current budget for this device; brightness is scaled to stay within it.
        /// Budgets are per device: several separately declared devices on one supply each get
        /// the default, so set it explicitly when their total matters.
        #[default_display = "Current::Milliamps(250)"]
        max_current: expr = $crate::led_strip::MAX_CURRENT_DEFAULT,
        /// Color correction curve.
        #[default_display = "Gamma::Srgb"]
        gamma: expr = $crate::led_strip::Gamma::Srgb,
        /// Maximum number of animation frames; `0` disables animation.
        #[default_display = "16"]
        max_frames: expr = $crate::led_strip::MAX_FRAMES_DEFAULT,
    }

    generate {
        // A one-member `led_strips!` group whose member is a panel; the group stays out
        // of the docs.
        $crate::led_strip::led_strips! {
            #[doc(hidden)]
            $decl.vis $ident($decl.name, Group) {
                pio: $decl.pio,

                $decl.attrs
                #[doc = $decl.doc]
                $decl.name {
                    pin: $decl.pin,
                    len: $decl.led_layout.len(),
                    max_current: $decl.max_current,
                    dma: $decl.dma,
                    gamma: $decl.gamma,
                    max_frames: $decl.max_frames,
                    led2d: { led_layout: $decl.led_layout, font: $decl.font },
                },
            }
        }

        impl $decl.name {
            /// Creates the LED panel and spawns its background task.
            ///
            /// The `pin`, `pio`, and `dma` arguments must be the GPIO pin, PIO resource,
            /// and DMA channel named in the macro. See the
            /// [led2d module documentation](mod@device_envoy_rp::led2d) for usage.
            pub fn new(
                pin: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pin>>,
                pio: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.pio>>,
                dma: impl Into<::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$decl.dma>>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<Self> {
                let (led2d,) = $ident($decl.name, Group)::new(pio, pin, dma, spawner)?;
                Ok(led2d)
            }
        }
    }
}
