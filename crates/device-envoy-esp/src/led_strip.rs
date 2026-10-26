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
//! - [`LedStrip`] - Core trait defining the LED strip API surface.
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
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p);
//!     let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
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
//! #     let err = example(spawner).await.unwrap_err();
//! #     panic!("{err:?}");
//! # }
//! async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
//!     init_and_start!(p);
//!     let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;
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

/// Drive the LEDs from an [RMT resource](crate#glossary).
#[derive(Clone, Copy, Debug)]
pub struct Rmt;

/// Drive the LEDs from an [SPI resource](crate#glossary).
#[derive(Clone, Copy, Debug)]
pub struct Spi;

/// An output engine for WS2812 LEDs: [`Rmt`] or [`Spi`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an LED engine on this chip",
    note = "chips without an RMT peripheral support only `Spi`"
)]
pub trait LedEngine {
    #[doc(hidden)]
    type Driver<const LEDS: usize, const BUF: usize>: Ws2812Write<LEDS>;
    #[doc(hidden)]
    const BUF_UNITS_PER_LED: usize;
    #[doc(hidden)]
    const USES_RESET_GAP: bool;
}

#[cfg(all(target_os = "none", esp_has_rmt))]
impl LedEngine for Rmt {
    type Driver<const LEDS: usize, const BUF: usize> = RmtWs2812<'static, LEDS, BUF>;
    const BUF_UNITS_PER_LED: usize = 24;
    const USES_RESET_GAP: bool = false;
}

#[cfg(target_os = "none")]
impl LedEngine for Spi {
    type Driver<const LEDS: usize, const BUF: usize> = spi::SpiWs2812<'static, LEDS, BUF>;
    const BUF_UNITS_PER_LED: usize = 9;
    const USES_RESET_GAP: bool = true;
}

/// The engine a strip uses when its declaration names none: RMT where the chip has it.
#[cfg(esp_has_rmt)]
pub type DefaultEngine = Rmt;
/// The engine a strip uses when its declaration names none: RMT where the chip has it.
#[cfg(not(esp_has_rmt))]
pub type DefaultEngine = Spi;

/// Transmit-buffer length for `leds` pixels: RMT pulses plus an end marker, or SPI bytes
/// plus the reset gap.
#[doc(hidden)]
#[must_use]
pub const fn engine_buffer_len<E: LedEngine>(leds: usize, reset_us: u32) -> usize {
    if E::USES_RESET_GAP {
        leds * E::BUF_UNITS_PER_LED + spi::reset_bytes_for_us(reset_us)
    } else {
        leds * E::BUF_UNITS_PER_LED + 1
    }
}

/// Writes one frame of WS2812 pixels to the wire.
#[doc(hidden)]
pub trait Ws2812Write<const LEDS: usize> {
    /// Why a write failed.
    type Error: core::fmt::Debug;
    /// Sends `frame` to the LEDs.
    fn write(&mut self, frame: &Frame1d<LEDS>) -> core::result::Result<(), Self::Error>;
}

/// The peripheral a strip's `new` takes to drive its LEDs, chosen by the strip's engine:
///
/// | Engine | Pass | Example |
/// | --- | --- | --- |
/// | [`Rmt`] | an RMT channel | `rmt80.channel0` |
/// | [`Spi`] | an SPI peripheral | `p.SPI2` |
///
/// This trait is sealed: these are its only implementations.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot drive `{E}` LEDs",
    label = "expected the peripheral for this strip's `{E}` engine",
    note = "an `Rmt` strip takes an RMT channel, such as `rmt80.channel0`; \
            an `Spi` strip takes an SPI peripheral, such as `p.SPI2`"
)]
pub trait EngineResource<E: LedEngine>: engine_resource_sealed::Sealed<E> {
    /// Builds the driver for a strip of `LEDS` pixels on `pin`.
    #[doc(hidden)]
    fn into_driver<const LEDS: usize, const BUF: usize>(
        self,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        reset_us: u32,
    ) -> crate::Result<E::Driver<LEDS, BUF>>;
}

// Keeps `EngineResource` closed to the two implementations below.
mod engine_resource_sealed {
    pub trait Sealed<E> {}

    #[cfg(all(target_os = "none", esp_has_rmt))]
    impl<C> Sealed<super::Rmt> for C where C: esp_hal::rmt::TxChannelCreator<'static, esp_hal::Blocking> {}

    #[cfg(target_os = "none")]
    impl<S> Sealed<super::Spi> for S where S: esp_hal::spi::master::Instance + 'static {}
}

#[cfg(all(target_os = "none", esp_has_rmt))]
impl<C> EngineResource<Rmt> for C
where
    C: esp_hal::rmt::TxChannelCreator<'static, esp_hal::Blocking>,
{
    fn into_driver<const LEDS: usize, const BUF: usize>(
        self,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        _reset_us: u32,
    ) -> crate::Result<RmtWs2812<'static, LEDS, BUF>> {
        let channel = self
            .configure_tx(&crate::init_and_start::rmt::ws2812_tx_config())
            .map_err(crate::Error::RmtConfig)?
            .with_pin(pin);
        Ok(RmtWs2812::new(channel))
    }
}

#[cfg(target_os = "none")]
impl<S> EngineResource<Spi> for S
where
    S: esp_hal::spi::master::Instance + 'static,
{
    fn into_driver<const LEDS: usize, const BUF: usize>(
        self,
        pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'static>,
        reset_us: u32,
    ) -> crate::Result<spi::SpiWs2812<'static, LEDS, BUF>> {
        spi::SpiWs2812::new(self, pin, spi::reset_bytes_for_us(reset_us))
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
use embassy_futures::select::{Either, select};
#[cfg(target_os = "none")]
use embassy_time::Timer;

#[cfg(all(target_os = "none", esp_has_rmt))]
use esp_hal::gpio::Level;
#[cfg(all(target_os = "none", esp_has_rmt))]
use esp_hal::rmt::{Channel, PulseCode, Tx};

// WS2812 timing at 80 MHz RMT clock with clk_divider=4 → 50 ns per tick.
//   T0H =  0.4 µs  → 8 ticks     T0L = 0.85 µs → 17 ticks
//   T1H =  0.8 µs  → 16 ticks    T1L = 0.45 µs →  9 ticks
#[cfg(all(target_os = "none", esp_has_rmt))]
const BIT0: PulseCode = PulseCode::new(Level::High, 8, Level::Low, 17);
#[cfg(all(target_os = "none", esp_has_rmt))]
const BIT1: PulseCode = PulseCode::new(Level::High, 16, Level::Low, 9);

/// WS2812 driver backed by an ESP32 RMT TX channel.
///
/// `LEDS` is the number of LED pixels; `PULSES` must equal `LEDS * 24 + 1`.
/// Both are generated as concrete `const` values by the [`led_strip!`](macro@crate::led_strip) macro,
/// so no `generic_const_exprs` is required.
///
/// The pulse buffer is a **field** of this struct so that it lives in BSS /
/// static memory rather than on the stack.
#[cfg(all(target_os = "none", esp_has_rmt))]
// Must be `pub` for macro expansion at foreign call sites.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
pub struct RmtWs2812<'d, const LEDS: usize, const PULSES: usize> {
    channel: Option<Channel<'d, esp_hal::Blocking, Tx>>,
    pulse_buf: [PulseCode; PULSES],
}

#[cfg(all(target_os = "none", esp_has_rmt))]
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
    pub fn write(&mut self, frame: &Frame1d<LEDS>) -> Result<(), Error> {
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

        let channel = self.channel.take().ok_or(Error::ChannelMissing)?;
        // On failure the HAL hands the channel back; keep it so the next frame can be sent.
        let transfer = match channel.transmit(&self.pulse_buf) {
            Ok(transfer) => transfer,
            Err((err, channel)) => {
                self.channel = Some(channel);
                return Err(Error::TransmitStart(err));
            }
        };
        match transfer.wait() {
            Ok(channel) => {
                self.channel = Some(channel);
                Ok(())
            }
            Err((err, channel)) => {
                self.channel = Some(channel);
                Err(Error::Transmit(err))
            }
        }
    }
}

#[cfg(all(target_os = "none", esp_has_rmt))]
impl<const LEDS: usize, const PULSES: usize> Ws2812Write<LEDS> for RmtWs2812<'_, LEDS, PULSES> {
    type Error = Error;

    fn write(&mut self, frame: &Frame1d<LEDS>) -> core::result::Result<(), Error> {
        RmtWs2812::write(self, frame)
    }
}

#[cfg(all(target_os = "none", esp_has_rmt))]
#[doc(hidden)]
/// Errors returned by [`RmtWs2812::write`].
#[derive(Debug)]
pub enum Error {
    /// Channel was already consumed and not recovered (internal logic error).
    ChannelMissing,
    /// RMT peripheral could not start the transfer.
    TransmitStart(esp_hal::rmt::Error),
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
pub async fn led_strip_device_loop<const LEDS: usize, const MAX_FRAMES: usize>(
    mut driver: impl Ws2812Write<LEDS>,
    command_signal: &'static LedStripCommandSignal<LEDS, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> ! {
    // Start with all LEDs off.
    write_or_warn(&mut driver, &Frame1d::new());

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
                write_or_warn(&mut driver, &frame);
                // Hold until the next command arrives — handled at the top of the loop.
            }
            Command::Animate(sequence) => {
                // Loop the animation sequence until interrupted by a new command.
                'animate: loop {
                    for (mut frame, duration) in sequence.iter().cloned() {
                        apply_correction(&mut frame, combo_table);
                        write_or_warn(&mut driver, &frame);
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

/// Sends `frame`, logging a failure instead of dropping it. A background task has no caller
/// to return the error to, and the next frame is still worth sending.
#[cfg(target_os = "none")]
fn write_or_warn<const LEDS: usize>(driver: &mut impl Ws2812Write<LEDS>, frame: &Frame1d<LEDS>) {
    if let Err(error) = driver.write(frame) {
        log::warn!("LED strip write failed: {error:?}");
    }
}

// ============================================================================
// led_strip! macro
// ============================================================================

macro_schema::define! {
    /// The LED strip type behind `led_strip!` and each `led2d!` panel: the struct, its
    /// constructor, the `LedStrip` impl, and its background task. Public because it expands in downstream crates, but not user-facing API.
    #[cfg(target_os = "none")]
    #[doc(hidden)]
    pub __led_strip_type {
        /// GPIO pin for LED data.
        pin: ident,
        /// Number of LEDs.
        len: expr,
        /// Electrical current budget.
        max_current: expr,
        /// Output engine type.
        engine: ty,
        /// Color curve.
        gamma: expr,
        /// Maximum number of animation frames.
        max_frames: expr,
        /// WS2812 reset interval in microseconds; only the `Spi` engine uses it.
        reset_us?: expr,
    }

    generate {
        // The engine is a type: the one named, or the chip's default. Everything that
        // differs between engines goes through its `LedEngine` and `EngineResource` impls.
        type $ident($decl.name, Engine) = $decl.engine;

        // Evaluated here, in the caller's scope, so `len` and `reset_us` may name the
        // caller's items.
        const $upper($decl.name, _LEDS): usize = $decl.len;
        const $upper($decl.name, _RESET_US): u32 =
            $if let Some(interval) = $decl.reset_us { $interval } else { $crate::led_strip::spi::RESET_US_DEFAULT };
        const $upper($decl.name, _BUF): usize =
            $crate::led_strip::engine_buffer_len::<$ident($decl.name, Engine)>(
                $upper($decl.name, _LEDS),
                $upper($decl.name, _RESET_US),
            );

        static $upper($decl.name, _STATIC): $crate::led_strip::LedStripStatic<
            { $upper($decl.name, _LEDS) },
            $const_arg($decl.max_frames),
        > = $crate::led_strip::LedStripHandle::new_static();

        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name {
            inner: $crate::led_strip::LedStripHandle<{ $upper($decl.name, _LEDS) }, $const_arg($decl.max_frames)>,
        }

        impl $decl.name {
            /// Number of pixels in this strip.
            pub const LEN: usize = $upper($decl.name, _LEDS);
            /// Maximum number of animation frames.
            pub const MAX_FRAMES: usize = $decl.max_frames;
            /// Maximum per-channel brightness (0–255) computed from `max_current`.
            pub const MAX_BRIGHTNESS: u8 = <$crate::led_strip::Current>::max_brightness(
                $decl.max_current,
                $upper($decl.name, _LEDS) as u32 * 60,
            );
            /// Combined gamma and brightness lookup table.
            const COMBO_TABLE: [u8; 256] =
                $crate::led_strip::generate_combo_table($decl.gamma, Self::MAX_BRIGHTNESS);

            /// Creates the strip and spawns its background task. `resource` is the peripheral
            /// for the strip's engine: an RMT channel (such as `rmt80.channel0`) for `Rmt`, or
            /// an SPI peripheral (such as `p.SPI2`) for `Spi`. See
            /// [`EngineResource`](device_envoy_esp::led_strip::EngineResource).
            pub fn new(
                pin: $crate::esp_hal::peripherals::$decl.pin<'static>,
                resource: impl $crate::led_strip::EngineResource<$ident($decl.name, Engine)>,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<&'static Self> {
                static INSTANCE: $crate::__static_cell::StaticCell<$decl.name> =
                    $crate::__static_cell::StaticCell::new();
                static COMBO: $crate::__static_cell::StaticCell<[u8; 256]> =
                    $crate::__static_cell::StaticCell::new();

                let combo_table: &'static [u8; 256] = COMBO.init(Self::COMBO_TABLE);
                let driver = resource.into_driver::<{ $upper($decl.name, _LEDS) }, { $upper($decl.name, _BUF) }>(
                    pin,
                    $upper($decl.name, _RESET_US),
                )?;
                let strip_static = &$upper($decl.name, _STATIC);
                spawner.spawn(
                    $snake($decl.name, _device_task)(driver, strip_static, combo_table)
                        .map_err($crate::Error::TaskSpawn)?,
                );
                Ok(INSTANCE.init($decl.name {
                    inner: $crate::led_strip::LedStripHandle::new(strip_static),
                }))
            }
        }

        impl $crate::led_strip::LedStrip<$const_arg($decl.len)> for $decl.name {
            const MAX_FRAMES: usize = $decl.max_frames;
            const MAX_BRIGHTNESS: u8 = Self::MAX_BRIGHTNESS;

            fn write_frame(&self, frame: $crate::led_strip::Frame1d<$const_arg($decl.len)>) {
                $crate::led_strip::__write_frame(self.inner.__command_signal(), frame);
            }

            fn animate<I>(&self, frames: I)
            where
                I: IntoIterator,
                I::Item: ::core::borrow::Borrow<(
                    $crate::led_strip::Frame1d<$const_arg($decl.len)>,
                    ::embassy_time::Duration,
                )>,
            {
                $crate::led_strip::__animate(self.inner.__command_signal(), frames);
            }
        }

        #[::embassy_executor::task]
        async fn $snake($decl.name, _device_task)(
            driver: <$ident($decl.name, Engine) as $crate::led_strip::LedEngine>::Driver<
                { $upper($decl.name, _LEDS) },
                { $upper($decl.name, _BUF) },
            >,
            strip_static: &'static $crate::led_strip::LedStripStatic<
                { $upper($decl.name, _LEDS) },
                $const_arg($decl.max_frames),
            >,
            combo_table: &'static [u8; 256],
        ) {
            $crate::led_strip::led_strip_device_loop(driver, strip_static.command_signal(), combo_table)
                .await;
        }
    }
}

macro_schema::define! {
    ///
    /// # Related Macros
    ///
    /// - [`led2d!`](macro@crate::led2d) — For 2-dimensional LED panels
    #[cfg(target_os = "none")]
    pub led_strip {
        /// GPIO pin for LED data, for example `GPIO8`.
        pin: ident,
        /// Number of LEDs (pixels).
        len: expr,
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
        /// WS2812 reset interval in microseconds. Only the `Spi` engine uses it; `Rmt`
        /// ignores it.
        #[default_display = "`60` (SPI only)"]
        reset_us: expr = $crate::led_strip::spi::RESET_US_DEFAULT,
    }

    generate {
        $crate::led_strip::__led_strip_type! {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis $decl.name {
                pin: $decl.pin,
                len: $decl.len,
                max_current: $decl.max_current,
                engine: $decl.engine,
                gamma: $decl.gamma,
                max_frames: $decl.max_frames,
                reset_us: $decl.reset_us,
            }
        }
    }
}

// ============================================================================
// SPI sub-module
// ============================================================================

#[cfg(target_os = "none")]
#[doc(hidden)]
pub mod spi;
