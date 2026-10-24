//! ESP32 SPI driver for NeoPixel-style (WS2812) LED strips.
//!
//! Used internally by `led_strip!` and `led2d!` when `engine: Spi`.

use super::Frame1d;

/// WS2812-over-SPI bitrate in Hz.
///
/// 2.4 MHz with 3 SPI bits per WS2812 bit gives 1.25 us WS2812 bit time.
pub const WS2812_SPI_HZ: u32 = 2_400_000;

/// Default WS2812 reset/latch interval for SPI output, in microseconds.
pub const RESET_US_DEFAULT: u32 = 60;

/// Convert a WS2812 reset/latch interval (microseconds) into trailing SPI zero bytes.
///
/// This uses ceiling division so the generated tail is never shorter than requested.
#[must_use]
pub const fn reset_bytes_for_us(reset_us: u32) -> usize {
    // bytes = ceil(reset_us * WS2812_SPI_HZ / 8_000_000)
    ((reset_us as u64 * WS2812_SPI_HZ as u64 + 7_999_999) / 8_000_000) as usize
}

/// Trailing zero bytes clocked after frame payload to satisfy the WS2812 reset/latch interval.
///
/// Defaults to [`RESET_US_DEFAULT`] converted at [`WS2812_SPI_HZ`].
pub const RESET_BYTES: usize = reset_bytes_for_us(RESET_US_DEFAULT);

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
        reset_bytes: usize,
    ) -> crate::Result<Self> {
        assert_eq!(
            BYTES,
            LEDS * 9 + reset_bytes,
            "BYTES must equal LEDS * 9 + reset_bytes; this is enforced by led_strip!"
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
    pub fn write(&mut self, frame: &Frame1d<LEDS>) -> Result<(), Error> {
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

        self.spi.write(&self.tx_buf).map_err(Error::Transmit)?;
        Ok(())
    }
}

impl<const LEDS: usize, const BYTES: usize> super::Ws2812Write<LEDS>
    for SpiWs2812<'_, LEDS, BYTES>
{
    type Error = Error;

    fn write(&mut self, frame: &Frame1d<LEDS>) -> core::result::Result<(), Error> {
        SpiWs2812::write(self, frame)
    }
}

/// Errors returned by [`SpiWs2812::write`].
#[derive(Debug)]
pub enum Error {
    /// SPI transmit failed.
    Transmit(esp_hal::spi::Error),
}
