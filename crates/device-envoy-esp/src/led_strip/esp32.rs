//! ESP32 RMT driver for NeoPixel-style (WS2812) LED strips, plus the
//! [`led_strip!`] macro that generates a fully async, Embassy-based strip
//! controller.
//!
//! See the [`led_strip!`] macro docs for the primary usage example.

use embassy_futures::select::{select, Either};
use embassy_time::Timer;
use esp_hal::gpio::Level;
use esp_hal::rmt::{Channel, PulseCode, Tx};

use super::{apply_correction, Command, Frame1d, LedStripCommandSignal};

// ============================================================================
// RmtWs2812: sync RMT-based WS2812 driver
// ============================================================================

// WS2812 timing at 80 MHz RMT clock with clk_divider=4 → 50 ns per tick.
//   T0H =  0.4 µs  → 8 ticks     T0L = 0.85 µs → 17 ticks
//   T1H =  0.8 µs  → 16 ticks    T1L = 0.45 µs →  9 ticks
const BIT0: PulseCode = PulseCode::new(Level::High, 8, Level::Low, 17);
const BIT1: PulseCode = PulseCode::new(Level::High, 16, Level::Low, 9);

/// WS2812 driver backed by an ESP32 RMT TX channel.
///
/// `LEDS` is the number of LED pixels; `PULSES` must equal `LEDS * 24 + 1`.
/// Both are generated as concrete `const` values by the [`led_strip!`] macro,
/// so no `generic_const_exprs` is required.
///
/// The pulse buffer is a **field** of this struct so that it lives in BSS /
/// static memory rather than on the stack.
pub struct RmtWs2812<'d, const LEDS: usize, const PULSES: usize> {
    channel: Option<Channel<'d, esp_hal::Blocking, Tx>>,
    pulse_buf: [PulseCode; PULSES],
}

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
/// matching [`LedStrip`][super::LedStrip] handle.
///
/// `#[doc(hidden)]` — called exclusively from macro-generated task code.
#[doc(hidden)]
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

/// Generate a fully async WS2812 LED strip controller for ESP32.
///
/// # Syntax
///
/// ```rust,no_run
/// use device_envoy_esp32::led_strip::esp32::led_strip;
///
/// led_strip! {
///     MyStrip {
///         len: 8,
///         max_current: device_envoy_esp32::led_strip::Current::Milliamps(1000),
///     }
/// }
/// ```
///
/// Optional fields and their defaults:
///
/// | Field | Default |
/// |---|---|
/// | `engine` | [`Engine::Rmt`][`super::Engine::Rmt`] |
/// | `gamma` | [`Gamma::Srgb`][`super::Gamma::Srgb`] |
/// | `max_frames` | `16` |
///
/// # What gets generated
///
/// - A `mod my_strip` containing `const LEDS`, `const PULSES = LEDS*24+1`.
/// - A struct `MyStrip` that derefs to [`LedStrip<LEDS, MAX_FRAMES>`][`super::LedStrip`].
/// - `MyStrip::new(channel, spawner) -> Result<&'static MyStrip>`, which
///   consumes a configured TX channel and spawns the background device task.
/// - `MyStrip::MAX_BRIGHTNESS`, `MY_STRIP::MAX_FRAMES`, `MyStrip::LEN`.
///
/// # `'static` requirement
///
/// `new()` returns `&'static MyStrip`. The static storage is hidden inside
/// a function-scoped `static` via [`static_cell::StaticCell`]. You only need
/// to call `new()` once.
#[macro_export]
macro_rules! led_strip {
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
            len: $len:expr,
            max_current: $max_current:expr,
            engine: device_envoy_esp32::led_strip::Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32_spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32_spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: $crate::led_strip::Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32_spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: device_envoy_esp32::led_strip::Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: $crate::led_strip::Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr,
            engine: $engine:path
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        compile_error!("led_strip! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        $name:ident {
            len: $len:expr,
            max_current: $max_current:expr
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::esp32::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [],
            [],
        }
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
        $len:expr,
        $max_current:expr,
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
        [$($led2d_layout:expr)?],
        [$($led2d_font:expr)?],
    ) => {
        $crate::__led_strip_impl!{
            name        = $name,
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
    ($leds:expr, $max_frames:expr, [$led_layout:expr], [$font:expr]) => {
        pub const LED_LAYOUT: $crate::led2d::LedLayout<
            { $leds },
            { $led_layout.width() },
            { $led_layout.height() },
        > = $led_layout;
        pub const WIDTH: usize = $led_layout.width();
        pub const HEIGHT: usize = $led_layout.height();
        pub const FONT: $crate::led2d::Led2dFont = $font;

        pub fn write_frame2d(
            &self,
            frame: $crate::led2d::Frame2d<{ $led_layout.width() }, { $led_layout.height() }>,
        ) -> $crate::Result<()> {
            let led2d = $crate::led2d::Led2d::new(&self.inner, &Self::LED_LAYOUT);
            led2d.write_frame(frame)
        }

        pub fn animate2d<I>(&self, frames: I) -> $crate::Result<()>
        where
            I: IntoIterator,
            I::Item: ::core::borrow::Borrow<(
                $crate::led2d::Frame2d<{ $led_layout.width() }, { $led_layout.height() }>,
                embassy_time::Duration,
            )>,
        {
            let led2d = $crate::led2d::Led2d::new(&self.inner, &Self::LED_LAYOUT);
            led2d.animate(frames)
        }

        pub fn write_text_to_frame(
            &self,
            text: &str,
            colors: &[$crate::led_strip::RGB8],
            frame: &mut $crate::led2d::Frame2d<{ $led_layout.width() }, { $led_layout.height() }>,
        ) -> $crate::Result<()> {
            $crate::led2d::render_text_to_frame(
                frame,
                &Self::FONT.to_font(),
                text,
                colors,
                Self::FONT.spacing_reduction(),
            )
        }

        pub fn write_text(
            &self,
            text: &str,
            colors: &[$crate::led_strip::RGB8],
        ) -> $crate::Result<()> {
            let mut frame =
                $crate::led2d::Frame2d::<{ $led_layout.width() }, { $led_layout.height() }>::new();
            self.write_text_to_frame(text, colors, &mut frame)?;
            self.write_frame2d(frame)
        }
    };
    ($_leds:expr, $_max_frames:expr, [], []) => {};
}

/// Core implementation macro. Do not call directly.
// Must be `pub` for macro expansion at foreign call site — not user-facing.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_impl {
    (
        name        = $name:ident,
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
                > = $crate::led_strip::LedStrip::new_static();

            // ------------------------------------------------------------------
            // Public struct.
            // ------------------------------------------------------------------
            pub struct $name {
                inner: $crate::led_strip::LedStrip<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >,
            }

            impl ::core::ops::Deref for $name {
                type Target = $crate::led_strip::LedStrip<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >;
                fn deref(&self) -> &Self::Target {
                    &self.inner
                }
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
                /// [`ws2812_tx_config`](crate::rmt::ws2812_tx_config).
                pub fn new(
                    pin: impl ::esp_hal::gpio::interconnect::PeripheralOutput<'static>,
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
                        .configure_tx(pin, $crate::rmt::ws2812_tx_config())
                        .map_err($crate::Error::Rmt)?;

                    let driver =
                        $crate::led_strip::esp32::RmtWs2812::<
                            { [<$name:snake _consts>]::LEDS },
                            { [<$name:snake _consts>]::PULSES },
                        >::new(channel);

                    let strip_static: &'static _ = &[<$name:snake:upper _STATIC>];

                    spawner
                        .spawn([<$name:snake _device_task>](driver, strip_static, combo_ref))
                        .map_err($crate::Error::TaskSpawn)?;

                    let instance = INSTANCE.init($name {
                        inner: $crate::led_strip::LedStrip::new(strip_static),
                    });
                    Ok(instance)
                }
            }

            // ------------------------------------------------------------------
            // Background task (embassy task function).
            // ------------------------------------------------------------------
            #[::embassy_executor::task]
            async fn [<$name:snake _device_task>](
                driver: $crate::led_strip::esp32::RmtWs2812<
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
                $crate::led_strip::esp32::led_strip_device_loop(
                    driver,
                    strip_static.command_signal(),
                    combo_table,
                )
                .await;
            }
        }
    };
}

// Re-export macros so they are visible from the `esp32` module path.
pub use crate::{
    __led2d_strip_methods, __led_strip_first_or_default, __led_strip_impl, __led_strip_inner,
    led_strip,
};
