//! ESP32 SPI driver for NeoPixel-style (WS2812) LED strips.
//!
//! Used internally by `led_strip!` when `engine: Engine::Spi`.

use embassy_futures::select::{select, Either};
use embassy_time::Timer;

use super::{apply_correction, Command, Frame1d, LedStripCommandSignal};

/// WS2812-over-SPI bitrate in Hz.
///
/// 2.4 MHz with 3 SPI bits per WS2812 bit gives 1.25 us WS2812 bit time.
pub const WS2812_SPI_HZ: u32 = 2_400_000;

/// Trailing zero bytes clocked after frame payload to satisfy the WS2812 reset/latch interval.
///
/// TODO0 Revisit reset tail length per board/chip and make configurable if needed.
pub const RESET_BYTES: usize = 18;

/// WS2812 driver backed by an ESP32 SPI master peripheral.
///
/// `LEDS` is the number of LED pixels.
/// `BYTES` must equal `LEDS * 9 + RESET_BYTES`.
pub struct SpiWs2812<'d, const LEDS: usize, const BYTES: usize> {
    spi: esp_hal::spi::master::Spi<'d, esp_hal::Blocking>,
    tx_buf: [u8; BYTES],
}

impl<'d, const LEDS: usize, const BYTES: usize> SpiWs2812<'d, LEDS, BYTES> {
    /// Create a new SPI WS2812 driver.
    ///
    /// Uses MOSI only; SCK and CS are left unconnected.
    ///
    /// # Errors
    /// Returns an error if SPI config is invalid.
    pub fn new(
        spi: impl esp_hal::spi::master::Instance + 'd,
        mosi_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'d>,
    ) -> crate::Result<Self> {
        assert_eq!(
            BYTES,
            LEDS * 9 + RESET_BYTES,
            "BYTES must equal LEDS * 9 + RESET_BYTES; this is enforced by led_strip!"
        );

        let config = esp_hal::spi::master::Config::default()
            .with_frequency(esp_hal::time::Rate::from_hz(WS2812_SPI_HZ))
            .with_mode(esp_hal::spi::Mode::_0);
        let spi = esp_hal::spi::master::Spi::new(spi, config)
            .map_err(crate::Error::SpiConfig)?
            .with_mosi(mosi_pin);

        Ok(Self {
            spi,
            tx_buf: [0u8; BYTES],
        })
    }

    /// Encode `frame` and transmit synchronously over SPI.
    pub fn write(&mut self, frame: &Frame1d<LEDS>) -> Result<(), SpiWritingError> {
        self.tx_buf.fill(0);

        let mut byte_index = 0usize;
        let mut bit_mask = 0x80u8;

        for pixel in frame.iter() {
            // WS2812 requires GRB MSB-first.
            let grb = ((pixel.g as u32) << 16) | ((pixel.r as u32) << 8) | (pixel.b as u32);
            for bit_index in 0..24 {
                let ws2812_bit = ((grb >> (23 - bit_index)) & 1) != 0;
                let symbol = if ws2812_bit { 0b110u8 } else { 0b100u8 };
                for symbol_bit_index in 0..3 {
                    let mask = 1 << (2 - symbol_bit_index);
                    if (symbol & mask) != 0 {
                        self.tx_buf[byte_index] |= bit_mask;
                    }
                    bit_mask >>= 1;
                    if bit_mask == 0 {
                        bit_mask = 0x80;
                        byte_index += 1;
                    }
                }
            }
        }

        self.spi
            .write(&self.tx_buf)
            .map_err(SpiWritingError::Transmit)?;
        Ok(())
    }
}

/// Errors returned by [`SpiWs2812::write`].
#[derive(Debug)]
pub enum SpiWritingError {
    /// SPI transmit failed.
    Transmit(esp_hal::spi::Error),
}

/// Asynchronous device loop for an SPI-backed WS2812 LED strip.
#[doc(hidden)]
pub async fn led_strip_spi_device_loop<
    'd,
    const LEDS: usize,
    const BYTES: usize,
    const MAX_FRAMES: usize,
>(
    mut driver: SpiWs2812<'d, LEDS, BYTES>,
    command_signal: &'static LedStripCommandSignal<LEDS, MAX_FRAMES>,
    combo_table: &'static [u8; 256],
) -> ! {
    let _ = driver.write(&Frame1d::new());
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
            }
            Command::Animate(sequence) => 'animate: loop {
                for (mut frame, duration) in sequence.iter().cloned() {
                    apply_correction(&mut frame, combo_table);
                    let _ = driver.write(&frame);
                    match select(Timer::after(duration), command_signal.wait()).await {
                        Either::First(_) => {}
                        Either::Second(new_command) => {
                            pending = Some(new_command);
                            break 'animate;
                        }
                    }
                }
                if let Some(new_command) = command_signal.try_take() {
                    pending = Some(new_command);
                    break 'animate;
                }
            },
        }
    }
}

/// Internal helper macro used by `led_strip!` for `Engine::Spi`.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_spi_inner {
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
        $crate::__led_strip_spi_impl!{
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

/// Core SPI implementation macro for `led_strip!`.
#[doc(hidden)]
#[macro_export]
macro_rules! __led_strip_spi_impl {
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
            mod [<$name:snake _consts>] {
                pub const LEDS: usize = $len;
                pub const BYTES: usize = LEDS * 9 + $crate::led_strip::spi::RESET_BYTES;
                pub const WORST_CASE_MA: u32 = LEDS as u32 * 60;
            }

            static [<$name:snake:upper _STATIC>]:
                $crate::led_strip::LedStripStatic<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                > = $crate::led_strip::LedStripEsp::new_static();

            pub struct $name {
                inner: $crate::led_strip::LedStripEsp<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >,
            }

            impl $name {
                pub const LEN: usize = [<$name:snake _consts>]::LEDS;
                pub const MAX_FRAMES: usize = $max_frames;
                pub const MAX_BRIGHTNESS: u8 = <$crate::led_strip::Current>::max_brightness(
                    $max_current,
                    [<$name:snake _consts>]::WORST_CASE_MA,
                );
                pub const COMBO_TABLE: [u8; 256] =
                    $crate::led_strip::generate_combo_table($gamma, Self::MAX_BRIGHTNESS);

                $crate::__led2d_strip_methods!(
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                    [$($led2d_layout)?],
                    [$($led2d_font)?]
                );

                /// Construct the strip controller from a MOSI pin and SPI peripheral.
                pub fn new(
                    pin: $crate::esp_hal::peripherals::$pin<'static>,
                    spi: impl ::esp_hal::spi::master::Instance + 'static,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<&'static Self> {
                    use ::static_cell::StaticCell;

                    static INSTANCE: StaticCell<$name> = StaticCell::new();
                    static COMBO: StaticCell<[u8; 256]> = StaticCell::new();

                    let combo_ref: &'static [u8; 256] = COMBO.init(<$name>::COMBO_TABLE);

                    let driver =
                        $crate::led_strip::spi::SpiWs2812::<
                            { [<$name:snake _consts>]::LEDS },
                            { [<$name:snake _consts>]::BYTES },
                        >::new(spi, pin)?;

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

            #[::embassy_executor::task]
            async fn [<$name:snake _device_task>](
                driver: $crate::led_strip::spi::SpiWs2812<
                    'static,
                    { [<$name:snake _consts>]::LEDS },
                    { [<$name:snake _consts>]::BYTES },
                >,
                strip_static: &'static $crate::led_strip::LedStripStatic<
                    { [<$name:snake _consts>]::LEDS },
                    { $max_frames },
                >,
                combo_table: &'static [u8; 256],
            ) {
                $crate::led_strip::spi::led_strip_spi_device_loop(
                    driver,
                    strip_static.command_signal(),
                    combo_table,
                )
                .await;
            }
        }
    };
}

// Re-export macros so they are visible from the `spi` module path.
pub use crate::{__led_strip_spi_impl, __led_strip_spi_inner};
