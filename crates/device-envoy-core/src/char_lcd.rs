//! A device abstraction for shared HD44780 character LCD protocol/state helpers.
//!
//! See the platform-specific crates (for example `device_envoy_rp::char_lcd` or
//! `device_envoy_esp::char_lcd`) for constructors and usage examples.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;

/// Messages sent to the character LCD device task.
#[derive(Clone, Debug)]
pub enum CharLcdMessage {
    /// Display a message for the specified duration (0 = until next message).
    Display {
        /// Text to render on the LCD.
        text: String<64>,
        /// Minimum display duration in milliseconds.
        duration_ms: u32,
    },
}

/// Static channel resources for character LCD command delivery.
pub struct CharLcdStatic(Channel<CriticalSectionRawMutex, CharLcdMessage, 8>);

impl CharLcdStatic {
    /// Creates static resources for the character LCD device.
    #[must_use]
    pub const fn new() -> Self {
        Self(Channel::new())
    }

    /// Queue one message to the LCD task.
    pub async fn send(&self, message: CharLcdMessage) {
        self.0.send(message).await;
    }

    /// Wait for the next message for the LCD task.
    pub async fn receive(&self) -> CharLcdMessage {
        self.0.receive().await
    }
}

/// Character LCD write adapter for platform crates.
pub trait CharLcdWrite {
    /// Write one byte to the configured LCD I2C expander.
    fn write(&mut self, address: u8, data: u8);
}

// PCF8574 pin mapping: P0=RS, P1=RW, P2=E, P3=Backlight, P4-P7=Data.
const LCD_BACKLIGHT: u8 = 0x08;
const LCD_ENABLE: u8 = 0x04;
const LCD_RS: u8 = 0x01;

/// HD44780 protocol driver over a byte-oriented I2C expander transport.
pub struct CharLcdDriver {
    address: u8,
}

impl CharLcdDriver {
    /// Creates a driver for the default PCF8574 backpack address (`0x27`).
    #[must_use]
    pub const fn new_default() -> Self {
        Self { address: 0x27 }
    }

    /// Initialize the LCD in 4-bit mode and clear it.
    pub async fn init(&mut self, char_lcd_write: &mut impl CharLcdWrite) {
        Timer::after_millis(50).await;

        self.write_nibble(char_lcd_write, 0x03, false).await;
        Timer::after_millis(5).await;
        self.write_nibble(char_lcd_write, 0x03, false).await;
        Timer::after_micros(150).await;
        self.write_nibble(char_lcd_write, 0x03, false).await;
        self.write_nibble(char_lcd_write, 0x02, false).await;

        // Function set: 4-bit, 2 lines, 5x8 font.
        self.write_byte(char_lcd_write, 0x28, false).await;
        // Display control: display on, cursor off, blink off.
        self.write_byte(char_lcd_write, 0x0C, false).await;
        // Clear display.
        self.write_byte(char_lcd_write, 0x01, false).await;
        Timer::after_millis(2).await;
        // Entry mode: increment cursor, no shift.
        self.write_byte(char_lcd_write, 0x06, false).await;
    }

    /// Process one queued LCD message.
    pub async fn process_message(
        &mut self,
        char_lcd_write: &mut impl CharLcdWrite,
        char_lcd_message: CharLcdMessage,
    ) {
        match char_lcd_message {
            CharLcdMessage::Display { text, duration_ms } => {
                self.clear(char_lcd_write).await;
                if let Some((line1, line2)) = text.as_str().split_once('\n') {
                    self.print(char_lcd_write, line1).await;
                    self.set_cursor(char_lcd_write, 1, 0).await;
                    self.print(char_lcd_write, line2).await;
                } else {
                    self.print(char_lcd_write, text.as_str()).await;
                }

                if duration_ms > 0 {
                    Timer::after_millis(duration_ms.into()).await;
                }
            }
        }
    }

    #[expect(clippy::arithmetic_side_effects, reason = "Bit operations")]
    async fn write_nibble(&mut self, char_lcd_write: &mut impl CharLcdWrite, nibble: u8, rs: bool) {
        let rs_bit = if rs { LCD_RS } else { 0 };
        let data = (nibble << 4) | LCD_BACKLIGHT | rs_bit;

        char_lcd_write.write(self.address, data | LCD_ENABLE);
        Timer::after_micros(1).await;
        char_lcd_write.write(self.address, data);
        Timer::after_micros(50).await;
    }

    async fn write_byte(&mut self, char_lcd_write: &mut impl CharLcdWrite, byte: u8, rs: bool) {
        self.write_nibble(char_lcd_write, (byte >> 4) & 0x0F, rs)
            .await;
        self.write_nibble(char_lcd_write, byte & 0x0F, rs).await;
    }

    async fn clear(&mut self, char_lcd_write: &mut impl CharLcdWrite) {
        self.write_byte(char_lcd_write, 0x01, false).await;
        Timer::after_millis(2).await;
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "Row/column values are small"
    )]
    async fn set_cursor(&mut self, char_lcd_write: &mut impl CharLcdWrite, row: u8, col: u8) {
        let address = match row {
            0 => col,
            1 => 0x40 + col,
            2 => 0x14 + col,
            3 => 0x54 + col,
            _ => 0,
        };
        self.write_byte(char_lcd_write, 0x80 | address, false).await;
    }

    async fn print(&mut self, char_lcd_write: &mut impl CharLcdWrite, text: &str) {
        for ch in text.bytes() {
            self.write_byte(char_lcd_write, ch, true).await;
        }
    }
}
