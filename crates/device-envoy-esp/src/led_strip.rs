#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "led_strip_simple",
        "docs/assets/led_strip_simple.png"
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
//! - [`led_strip!`](macro@crate::led_strip) - Macro to generate an LED-strip struct type (includes syntax details).
//! - [`LedStrip`](`crate::led_strip::LedStrip`) - Core trait defining the LED strip API surface.
//! - [`LedStripGenerated`](led_strip_generated::LedStripGenerated) - Sample generated strip type showing the constructor path.
//! - [`Frame1d`] - 1D pixel array used to describe LED strip patterns.
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
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::{Result, init_and_start, led_strip, led_strip::{Frame1d, LedStrip as _, colors}};
//!
//! // Define LedStripSimple, a struct type for an 8-LED strip on GPIO8.
//! led_strip! {
//!     LedStripSimple {
//!         pin: GPIO8,  // GPIO pin for LED data
//!         len: 8,      // 8 LEDs
//!         // other inputs set to their defaults
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
//!     // Create a LedStripSimple instance.
//!     let led_strip_simple = LedStripSimple::new(p.GPIO8, rmt80.channel0, spawner)?;
//!
//!     // Create and write a frame with alternating blue and gray pixels.
//!     let mut frame = Frame1d::new();
//!     for pixel_index in 0..LedStripSimple::LEN {
//!         // Directly index into the frame buffer.
//!         frame[pixel_index] = [colors::BLUE, colors::GRAY][pixel_index % 2];
//!     }
//!
//!     // Display the frame on the LED strip (until replaced).
//!     led_strip_simple.write_frame(frame);
//!
//!     core::future::pending().await
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
//! # use core::convert::Infallible;
//! # use esp_backtrace as _;
//! use device_envoy_esp::{Result, init_and_start, led_strip, led_strip::{Current, Frame1d, Gamma, LedStrip as _, colors}};
//! use embassy_time::Duration;
//!
//! // Define LedStripAnimated, a struct type for a 96-LED strip on GPIO18.
//! // We change some defaults including setting a 1A power budget and disabling gamma correction.
//! led_strip! {
//!     LedStripAnimated {
//!         pin: GPIO18,                           // GPIO pin for LED data
//!         len: 96,                               // 96 LEDs
//!         max_current: Current::Milliamps(1000), // 1A power budget
//!         gamma: Gamma::Linear,                  // No color correction
//!         max_frames: 3,                         // Up to 3 animation frames
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
//!     let led_strip_animated = LedStripAnimated::new(p.GPIO18, rmt80.channel0, spawner)?;
//!
//!     // Create a sequence of frames and durations and then animate them (looping, until replaced).
//!     let frame_duration = Duration::from_millis(300);
//!     led_strip_animated.animate([
//!         (Frame1d::filled(colors::RED), frame_duration),
//!         (Frame1d::filled(colors::GREEN), frame_duration),
//!         (Frame1d::filled(colors::BLUE), frame_duration),
//!     ]);
//!
//!     core::future::pending().await
//! }
//! ```

pub use device_envoy_core::led_strip::*;
pub mod led_strip_generated;

/// Internal runtime handle for macro-generated LED strip types.
///
/// `#[doc(hidden)]` because this is implementation detail used by macro output.
#[doc(hidden)]
pub struct LedStripEsp<const N: usize, const MAX_FRAMES: usize> {
    command_signal: &'static LedStripCommandSignal<N, MAX_FRAMES>,
}

impl<const N: usize, const MAX_FRAMES: usize> LedStripEsp<N, MAX_FRAMES> {
    #[doc(hidden)]
    pub const fn new_static() -> LedStripStatic<N, MAX_FRAMES> {
        LedStripStatic::new_static()
    }

    #[doc(hidden)]
    pub fn new(led_strip_static: &'static LedStripStatic<N, MAX_FRAMES>) -> Self {
        Self {
            command_signal: led_strip_static.command_signal(),
        }
    }

    // Must be `pub` for macro expansion at foreign call sites — not user-facing.
    #[doc(hidden)]
    pub fn __command_signal(&self) -> &'static LedStripCommandSignal<N, MAX_FRAMES> {
        self.command_signal
    }
}

/// Tells whether to run LEDs from an [RMT resource](crate#glossary) or an
/// [SPI resource](crate#glossary).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine {
    /// Use an [RMT resource](crate#glossary).
    Rmt,
    /// Use an [SPI resource](crate#glossary).
    Spi,
}

impl Default for Engine {
    fn default() -> Self {
        Self::Rmt
    }
}

// Must be `pub` for macro expansion at foreign call sites.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
/// Default current budget used by [`led_strip!`](macro@crate::led_strip) when
/// `max_current` is omitted.
pub const CURRENT_DEFAULT: Current = Current::Milliamps(250);

// ============================================================================
// RMT driver (ESP32-specific)
// ============================================================================

#[cfg(target_os = "none")]
use embassy_futures::select::{select, Either};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(target_os = "none")]
use esp_hal::gpio::Level;
#[cfg(target_os = "none")]
use esp_hal::rmt::{Channel, PulseCode, Tx};

// WS2812 timing at 80 MHz RMT clock with clk_divider=4 → 50 ns per tick.
//   T0H =  0.4 µs  → 8 ticks     T0L = 0.85 µs → 17 ticks
//   T1H =  0.8 µs  → 16 ticks    T1L = 0.45 µs →  9 ticks
#[cfg(target_os = "none")]
const BIT0: PulseCode = PulseCode::new(Level::High, 8, Level::Low, 17);
#[cfg(target_os = "none")]
const BIT1: PulseCode = PulseCode::new(Level::High, 16, Level::Low, 9);

/// WS2812 driver backed by an ESP32 RMT TX channel.
///
/// `LEDS` is the number of LED pixels; `PULSES` must equal `LEDS * 24 + 1`.
/// Both are generated as concrete `const` values by the [`led_strip!`](macro@crate::led_strip) macro,
/// so no `generic_const_exprs` is required.
///
/// The pulse buffer is a **field** of this struct so that it lives in BSS /
/// static memory rather than on the stack.
#[cfg(target_os = "none")]
// Must be `pub` for macro expansion at foreign call sites.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
pub struct RmtWs2812<'d, const LEDS: usize, const PULSES: usize> {
    channel: Option<Channel<'d, esp_hal::Blocking, Tx>>,
    pulse_buf: [PulseCode; PULSES],
}

#[cfg(target_os = "none")]
impl<'d, const LEDS: usize, const PULSES: usize> RmtWs2812<'d, LEDS, PULSES> {
    /// Create a new driver, taking ownership of an RMT TX channel.
    ///
    /// Called internally by the `led_strip!`-generated `new()`. The channel
    /// must be configured with clock divider 4, no carrier, and idle-low.
    #[must_use]
    pub fn new(channel: Channel<'d, esp_hal::Blocking, Tx>) -> Self {
        assert_eq!(
            PULSES,
            LEDS * 24 + 1,
            "PULSES must equal LEDS * 24 + 1; this is enforced by led_strip!"
        );
        Self {
            channel: Some(channel),
            pulse_buf: [PulseCode::end_marker(); PULSES],
        }
    }

    /// Encode `frame` into the pulse buffer and transmit synchronously.
    ///
    /// GRB byte order (required by WS2812) is applied here. Gamma/brightness
    /// correction must be applied to the frame before calling this method.
    pub fn write(&mut self, frame: &Frame1d<LEDS>) -> Result<(), WritingError> {
        // Encode each pixel as 24 bits in GRB MSB-first order.
        for (led_index, pixel) in frame.iter().enumerate() {
            let grb: u32 = ((pixel.g as u32) << 16) | ((pixel.r as u32) << 8) | (pixel.b as u32);
            for bit_index in 0..24 {
                let bit = (grb >> (23 - bit_index)) & 1;
                self.pulse_buf[led_index * 24 + bit_index] = if bit == 1 { BIT1 } else { BIT0 };
            }
        }
        // Final slot is always the end marker. Written explicitly on every
        // transmit to guard against future refactoring.
        self.pulse_buf[LEDS * 24] = PulseCode::end_marker();

        let channel = self.channel.take().ok_or(WritingError::ChannelMissing)?;
        let transfer = channel
            .transmit(&self.pulse_buf)
            .map_err(|_| WritingError::TransmitStart)?;
        match transfer.wait() {
            Ok(channel) => {
                self.channel = Some(channel);
                Ok(())
            }
            Err((err, channel)) => {
                self.channel = Some(channel);
                Err(WritingError::Transmit(err))
            }
        }
    }
}

#[cfg(target_os = "none")]
#[doc(hidden)]
/// Errors returned by [`RmtWs2812::write`].
#[derive(Debug)]
pub enum WritingError {
    /// Channel was already consumed and not recovered (internal logic error).
    ChannelMissing,
    /// RMT peripheral could not start the transfer.
    TransmitStart,
    /// RMT peripheral reported an error during or after transfer.
    Transmit(esp_hal::rmt::Error),
}

// ============================================================================
// Device loop
// ============================================================================

/// Asynchronous device loop for a WS2812 LED strip.
///
/// Call this from an `embassy_executor::task` spawned by the generated
/// `new()` constructor. It runs forever, receiving [`Command`]s from the
/// matching [`LedStrip`] handle.
///
/// `#[doc(hidden)]` — called exclusively from macro-generated task code.
#[doc(hidden)]
#[cfg(target_os = "none")]
pub async fn led_strip_device_loop<
    'd,
    const LEDS: usize,
    const PULSES: usize,
    const MAX_FRAMES: usize,
>(
    mut driver: RmtWs2812<'d, LEDS, PULSES>,
    command_signal: &'static LedStripCommandSignal<LEDS, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> ! {
    // Start with all LEDs off.
    let _ = driver.write(&Frame1d::new());

    // `pending` carries a command that was received during animation into the
    // next iteration of the outer loop, avoiding recursion.
    let mut pending: Option<Command<LEDS, MAX_FRAMES>> = None;

    loop {
        let command = match pending.take() {
            Some(cmd) => cmd,
            None => command_signal.wait().await,
        };

        match command {
            Command::DisplayStatic(mut frame) => {
                apply_correction(&mut frame, combo_table);
                let _ = driver.write(&frame);
                // Hold until the next command arrives — handled at the top of the loop.
            }
            Command::Animate(sequence) => {
                // Loop the animation sequence until interrupted by a new command.
                'animate: loop {
                    for (mut frame, duration) in sequence.iter().cloned() {
                        apply_correction(&mut frame, combo_table);
                        let _ = driver.write(&frame);
                        match select(Timer::after(duration), command_signal.wait()).await {
                            Either::First(_) => {
                                // Timer elapsed — continue to next frame.
                            }
                            Either::Second(new_command) => {
                                // New command arrived mid-animation; carry it to the
                                // outer loop via `pending` rather than recurse.
                                pending = Some(new_command);
                                break 'animate;
                            }
                        }
                    }
                    // One full pass completed — check for a new command before
                    // looping the animation again (non-blocking).
                    if let Some(new_command) = command_signal.try_take() {
                        pending = Some(new_command);
                        break 'animate;
                    }
                }
            }
        }
    }
}

// ============================================================================
// led_strip! macro
// ============================================================================

/// Macro to generate an LED-strip struct type (includes syntax details).
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
///         max_current: <Current_expr>, // optional
///         engine: <Engine_expr>,       // optional
///         gamma: <Gamma_expr>,         // optional
///         max_frames: <usize_expr>,    // optional
///         reset_us: <u32_expr>,        // optional (SPI only)
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
/// - `max_current` — Electrical current budget (default: 250 mA)
/// - `engine` — Output engine (default: `Engine::Rmt`)
/// - `gamma` — Color curve (default: `Gamma::Srgb`)
/// - `max_frames` — Maximum number of animation frames (default: 16 frames)
/// - `reset_us` — WS2812 reset/latch interval in microseconds for `Engine::Spi` (default: 60)
///
/// `max_frames = 0` disables animation and allocates no frame storage; `write_frame()` is still supported.
///
#[doc = include_str!("docs/current_limiting_and_gamma.md")]
///
/// # Related Macros
///
/// - [`led2d!`](mod@crate::led2d) — For 2-dimensional LED panels
#[doc(hidden)]
#[macro_export]
macro_rules! led_strip {
    ($($tt:tt)*) => { $crate::__led_strip_entry! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led_strip!`] instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_entry {
    (
        $name:ident {
            $($before:tt)*
            led2d: { $($led2d_fields:tt)* }
            $($after:tt)*
        }
    ) => {
        compile_error!("led_strip! is 1D-only. Use led2d! for panel generation.");
    };
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [],
            len = [],
            max_current = [],
            engine = [],
            gamma = [],
            max_frames = [],
            reset_us = [],
            fields = [$($fields)*],
        }
    };
    (
        $vis:vis $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__paste! {
            $crate::__led_strip_entry! {
                [<__ $name _visibility_inner>] {
                    $($fields)*
                }
            }
            $vis type $name = [<__ $name _visibility_inner>];
        }
    };
}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use led_strip;

#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_collect_fields {
    (
        name = $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [],
    ) => {
        $crate::__led_strip_dispatch_engine!(
            $name,
            $pin,
            $len,
            $crate::__led_strip_max_current_or_default!([$($max_current)?]),
            [$($engine)?],
            [$($gamma)?],
            [$($max_frames)?],
            [$($reset_us)?],
        );
    };
    (
        name = $name:ident,
        pin = [],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [],
    ) => {
        compile_error!("led_strip! missing required `pin` field");
    };
    (
        name = $name:ident,
        pin = [$pin:ident],
        len = [],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [],
    ) => {
        compile_error!("led_strip! missing required `len` field");
    };
    (
        name = $name:ident,
        pin = [],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$pin],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$already_pin:ident],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `pin` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$len],
            max_current = [$($max_current)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$already_len:expr],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `len` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$max_current],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$already_max_current:expr],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `max_current` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: $crate::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: $crate::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$already_engine:tt],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `engine` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [$($engine)?],
            gamma = [$gamma],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$already_gamma:expr],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `gamma` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [],
        reset_us = [$($reset_us:expr)?],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$max_frames],
            reset_us = [$($reset_us)?],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$already_max_frames:expr],
        reset_us = [$($reset_us:expr)?],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `max_frames` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [],
        fields = [reset_us: $reset_us:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led_strip_collect_fields!{
            name = $name,
            pin = [$($pin)?],
            len = [$($len)?],
            max_current = [$($max_current)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$reset_us],
            fields = [$($($rest)*)?],
        }
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$already_reset_us:expr],
        fields = [reset_us: $reset_us:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led_strip! duplicate `reset_us` field");
    };
    (
        name = $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        max_current = [$($max_current:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        fields = [$field:ident : $value:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!(
            "led_strip! unknown field; expected `pin`, `len`, `max_current`, `engine`, `gamma`, `max_frames`, or `reset_us`"
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_max_current_or_default {
    ([$max_current:expr]) => {
        $max_current
    };
    ([]) => {
        $crate::led_strip::CURRENT_DEFAULT
    };
}

/// Internal helper macro used by [`led_strip!`]. Do not call directly.
///
/// This is `pub` because the macro expansion happens at the call site in
/// downstream crates, so the token tree must be accessible from outside this
/// crate.
// Must be `pub` for macro expansion at foreign call site — not user-facing.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_inner {
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [$($led2d_layout:expr)?],
        [$($led2d_font:expr)?],
    ) => {
        $crate::__led_strip_impl!{
            name        = $name,
            pin         = $pin,
            len         = $len,
            max_current = $max_current,
            gamma       = $crate::__led_strip_first_or_default!(
                              [$($gamma)?],
                              $crate::led_strip::GAMMA_DEFAULT
                          ),
            max_frames  = $crate::__led_strip_first_or_default!(
                              [$($max_frames)?],
                              $crate::led_strip::MAX_FRAMES_DEFAULT
                          ),
            led2d_layout = [$($led2d_layout)?],
            led2d_font = [$($led2d_font)?],
        }
    };
}

/// Parse optional led_strip! fields (`engine`, `gamma`, `max_frames`, `reset_us`) in any order.
///
/// This is `pub` for downstream macro expansion at call sites.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_parse_options {
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
    ) => {
        $crate::__led_strip_dispatch_engine! {
            $name,
            $pin,
            $len,
            $max_current,
            [$($engine)*],
            [$($gamma)?],
            [$($max_frames)?],
            [$($reset_us)?],
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: Engine::Spi
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: $crate::led_strip::Engine::Spi
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: device_envoy_esp::led_strip::Engine::Spi
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: Engine::Rmt
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: $crate::led_strip::Engine::Rmt
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: device_envoy_esp::led_strip::Engine::Rmt
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)+],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: $ignored:path
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! duplicate `engine` field");
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        engine: $ignored:path
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        gamma: $gamma:expr
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [$($engine)*],
            gamma = [$gamma],
            max_frames = [$($max_frames)?],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$already_gamma:expr],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        gamma: $gamma:expr
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! duplicate `gamma` field");
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [],
        reset_us = [$($reset_us:expr)?],
        max_frames: $max_frames:expr
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [$($engine)*],
            gamma = [$($gamma)?],
            max_frames = [$max_frames],
            reset_us = [$($reset_us)?],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [$already_max_frames:expr],
        reset_us = [$($reset_us:expr)?],
        max_frames: $max_frames:expr
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! duplicate `max_frames` field");
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [],
        reset_us: $reset_us:expr
        $(, $($tail:tt)*)?
    ) => {
        $crate::__led_strip_parse_options! {
            name = $name,
            pin = $pin,
            len = $len,
            max_current = $max_current,
            engine = [$($engine)*],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            reset_us = [$reset_us],
            $($($tail)*)?
        }
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$already_reset_us:expr],
        reset_us: $reset_us:expr
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! duplicate `reset_us` field");
    };
    (
        name = $name:ident,
        pin = $pin:ident,
        len = $len:expr,
        max_current = $max_current:expr,
        engine = [$($engine:tt)*],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        reset_us = [$($reset_us:expr)?],
        $field:ident : $value:expr
        $(, $($tail:tt)*)?
    ) => {
        compile_error!("led_strip! unknown field; expected `engine`, `gamma`, `max_frames`, or `reset_us`");
    };
}

/// Dispatch parsed led_strip! options to RMT or SPI backend.
///
/// This is `pub` for downstream macro expansion at call sites.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_dispatch_engine {
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [Spi],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [$($reset_us:expr)?],
    ) => {
        $crate::led_strip::spi::__led_strip_spi_inner!{
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$($reset_us)?],
            [],
            [],
        }
    };
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [Rmt],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [$reset_us:expr],
    ) => {
        compile_error!("led_strip! `reset_us` is only supported with `engine: Engine::Spi`");
    };
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [Rmt],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [],
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [$reset_us:expr],
    ) => {
        compile_error!("led_strip! `reset_us` is only supported with `engine: Engine::Spi`");
    };
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $max_current:expr,
        [],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [],
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
}

/// Pick the first element of a bracketed list, or fall back to a default.
/// Only for use in `led_strip!` expansion. Do not call directly.
// Must be `pub` for macro expansion at foreign call site — not user-facing.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_first_or_default {
    ([$value:expr], $_default:expr) => {
        $value
    };
    ([],             $default:expr) => {
        $default
    };
}

/// Emit optional 2D-panel constants and methods on a generated strip type.
#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_strip_methods {
    ($_leds:expr, $max_frames:expr, [$led_layout:expr], [$font:expr]) => {
        /// Default font used by text helpers.
        pub const FONT: $crate::led2d::Led2dFont = $font;
        /// Panel width in pixels.
        pub const WIDTH: usize = $led_layout.width();
        /// Panel height in pixels.
        pub const HEIGHT: usize = $led_layout.height();
        /// Panel dimensions.
        pub const SIZE: $crate::led2d::Size =
            $crate::led2d::Frame2d::<{ $led_layout.width() }, { $led_layout.height() }>::SIZE;
        /// Top-left corner coordinate.
        pub const TOP_LEFT: $crate::led2d::Point =
            $crate::led2d::Frame2d::<{ $led_layout.width() }, { $led_layout.height() }>::TOP_LEFT;
        /// Top-right corner coordinate.
        pub const TOP_RIGHT: $crate::led2d::Point =
            $crate::led2d::Frame2d::<{ $led_layout.width() }, { $led_layout.height() }>::TOP_RIGHT;
        /// Bottom-left corner coordinate.
        pub const BOTTOM_LEFT: $crate::led2d::Point = $crate::led2d::Frame2d::<
            { $led_layout.width() },
            { $led_layout.height() },
        >::BOTTOM_LEFT;
        /// Bottom-right corner coordinate.
        pub const BOTTOM_RIGHT: $crate::led2d::Point = $crate::led2d::Frame2d::<
            { $led_layout.width() },
            { $led_layout.height() },
        >::BOTTOM_RIGHT;
    };
    ($_leds:expr, $_max_frames:expr, [], []) => {};
}

/// Emit optional LED2D trait impl for generated strip type.
#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_strip_trait_impl {
    ($name:ident, [$led_layout:expr], [$font:expr], $max_frames:expr) => {
        impl $crate::led2d::Led2d<{ $led_layout.width() }, { $led_layout.height() }>
            for &'static $name
        {
            const WIDTH: usize = $name::WIDTH;
            const HEIGHT: usize = $name::HEIGHT;
            const LEN: usize = $name::LEN;
            const SIZE: $crate::led2d::Size = $name::SIZE;
            const TOP_LEFT: $crate::led2d::Point = $name::TOP_LEFT;
            const TOP_RIGHT: $crate::led2d::Point = $name::TOP_RIGHT;
            const BOTTOM_LEFT: $crate::led2d::Point = $name::BOTTOM_LEFT;
            const BOTTOM_RIGHT: $crate::led2d::Point = $name::BOTTOM_RIGHT;
            const MAX_FRAMES: usize = $max_frames;
            const MAX_BRIGHTNESS: u8 = $name::MAX_BRIGHTNESS;
            const FONT: $crate::led2d::Led2dFont = $font;

            fn write_frame(
                &self,
                frame2d: $crate::led2d::Frame2d<{ $led_layout.width() }, { $led_layout.height() }>,
            ) {
                let led2d = $crate::led2d::Led2dEsp::new(*self, &$led_layout);
                $crate::led2d::Led2dStripBacked::write_frame(&led2d, frame2d);
            }

            fn animate<I>(&self, frames: I)
            where
                I: IntoIterator,
                I::Item: ::core::borrow::Borrow<(
                    $crate::led2d::Frame2d<{ $led_layout.width() }, { $led_layout.height() }>,
                    embassy_time::Duration,
                )>,
            {
                let led2d = $crate::led2d::Led2dEsp::new(*self, &$led_layout);
                $crate::led2d::Led2dStripBacked::animate(&led2d, frames);
            }
        }
    };
    ($_name:ident, [], [], $_max_frames:expr) => {};
}

/// Core implementation macro. Do not call directly.
// Must be `pub` for macro expansion at foreign call site — not user-facing.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_impl {
    (
        name        = $name:ident,
        pin         = $pin:ident,
        len         = $len:expr,
        max_current = $max_current:expr,
        gamma       = $gamma:expr,
        max_frames  = $max_frames:expr,
        led2d_layout = [$($led2d_layout:expr)?],
        led2d_font = [$($led2d_font:expr)?],
    ) => {
        ::paste::paste! {
            // ------------------------------------------------------------------
            // Module holding concrete const values for this strip instance.
            // Named after the struct in snake_case to avoid collisions.
            // ------------------------------------------------------------------
            mod [<$name:snake _consts>] {
                /// Number of LED pixels.
                pub const LEDS: usize = $len;
                /// Pulse buffer length: 24 bits per LED plus 1 end marker.
                pub const PULSES: usize = LEDS * 24 + 1;
                /// Maximum simultaneous-on current in milliamps at full brightness.
                pub const WORST_CASE_MA: u32 = LEDS as u32 * 60;
            }

            // ------------------------------------------------------------------
            // Static resources (signals etc.) — hidden from public docs.
            // ------------------------------------------------------------------
            static [<$name:snake:upper _STATIC>]:
                $crate::led_strip::LedStripStatic<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                > = $crate::led_strip::LedStripEsp::new_static();

            // ------------------------------------------------------------------
            // Public struct.
            // ------------------------------------------------------------------
            pub struct $name {
                inner: $crate::led_strip::LedStripEsp<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >,
            }

            impl $name {
                /// Number of pixels in this strip.
                pub const LEN: usize = [<$name:snake _consts>]::LEDS;

                /// Maximum number of animation frames.
                pub const MAX_FRAMES: usize = $max_frames;

                /// Maximum per-channel brightness (0–255) computed from
                /// `max_current`.
                pub const MAX_BRIGHTNESS: u8 = <$crate::led_strip::Current>::max_brightness(
                    $max_current,
                    [<$name:snake _consts>]::WORST_CASE_MA,
                );

                /// Combined gamma + brightness lookup table (const, zero cost).
                pub const COMBO_TABLE: [u8; 256] =
                    $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                $crate::__led2d_strip_methods!(
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                    [$($led2d_layout)?],
                    [$($led2d_font)?]
                );

                /// Construct the strip controller from an owned TX channel creator and GPIO pin.
                ///
                /// This configures a TX channel from a shared `rmt80` hub using
                /// [`ws2812_tx_config`](crate::init_and_start::rmt::ws2812_tx_config).
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin<'static>,
                    channel_creator: impl ::esp_hal::rmt::TxChannelCreator<
                        'static,
                        ::esp_hal::Blocking,
                    >,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    use ::static_cell::StaticCell;

                    static INSTANCE: StaticCell<$name> = StaticCell::new();
                    static COMBO: StaticCell<[u8; 256]> = StaticCell::new();

                    let combo_ref: &'static [u8; 256] =
                        COMBO.init(<$name>::COMBO_TABLE);

                    let channel = channel_creator
                        .configure_tx(pin, $crate::init_and_start::rmt::ws2812_tx_config())
                        .map_err($crate::Error::Rmt)?;

                    let driver =
                        $crate::led_strip::RmtWs2812::<
                            { [<$name:snake _consts>]::LEDS },
                            { [<$name:snake _consts>]::PULSES },
                        >::new(channel);

                    let strip_static: &'static _ = &[<$name:snake:upper _STATIC>];

                    spawner
                        .spawn([<$name:snake _device_task>](driver, strip_static, combo_ref))
                        .map_err($crate::Error::TaskSpawn)?;

                    let instance = INSTANCE.init($name {
                        inner: $crate::led_strip::LedStripEsp::new(strip_static),
                    });
                    Ok(instance)
                }
            }

            impl $crate::led_strip::LedStrip<{ [<$name:snake _consts>]::LEDS }> for $name {
                const MAX_FRAMES: usize = $max_frames;
                const MAX_BRIGHTNESS: u8 = Self::MAX_BRIGHTNESS;

                fn write_frame(
                    &self,
                    frame: $crate::led_strip::Frame1d<{ [<$name:snake _consts>]::LEDS }>,
                ) {
                    $crate::led_strip::__write_frame(self.inner.__command_signal(), frame);
                }

                fn animate<I>(&self, frames: I)
                where
                    I: IntoIterator,
                    I::Item: ::core::borrow::Borrow<(
                        $crate::led_strip::Frame1d<{ [<$name:snake _consts>]::LEDS }>,
                        embassy_time::Duration,
                    )>,
                {
                    $crate::led_strip::__animate(self.inner.__command_signal(), frames);
                }
            }

            $crate::__led2d_strip_trait_impl!(
                $name,
                [$($led2d_layout)?],
                [$($led2d_font)?],
                $max_frames
            );

            // ------------------------------------------------------------------
            // Background task (embassy task function).
            // ------------------------------------------------------------------
            #[::embassy_executor::task]
            async fn [<$name:snake _device_task>](
                driver: $crate::led_strip::RmtWs2812<
                    'static,
                    { [<$name:snake _consts>]::LEDS },
                    { [<$name:snake _consts>]::PULSES },
                >,
                strip_static: &'static $crate::led_strip::LedStripStatic<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >,
                combo_table: &'static [u8; 256],
            ) {
                $crate::led_strip::led_strip_device_loop(
                    driver,
                    strip_static.command_signal(),
                    combo_table,
                )
                .await;
            }
        }
    };
}

// ============================================================================
// SPI sub-module
// ============================================================================

#[cfg(target_os = "none")]
#[doc(hidden)]
pub mod spi;

// Re-export macros so they are visible from the `led_strip` module path.
pub use crate::{
    __led2d_strip_methods, __led2d_strip_trait_impl, __led_strip_dispatch_engine,
    __led_strip_first_or_default, __led_strip_impl, __led_strip_inner, __led_strip_parse_options,
};
