//! A device abstraction for shared HD44780 character LCD protocol/state helpers.
//!
//! See `device_envoy_rp::lcd_text` for constructors and usage examples.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use heapless::String;

/// Character LCD operation errors shared across platform crates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcdTextError {
    /// I2C write failed for the given 7-bit address.
    I2cWrite { address: u8 },
}

/// Messages sent to the character LCD device task.
#[derive(Clone, Debug)]
pub enum LcdTextMessage {
    /// Display a message for the specified duration (0 = until next message).
    Display {
        /// Text to render on the LCD.
        text: String<64>,
        /// Minimum display duration in milliseconds.
        duration_ms: u32,
    },
}

/// Static channel resources for character LCD command delivery.
pub struct LcdTextStatic {
    request_channel: Channel<CriticalSectionRawMutex, LcdTextMessage, 8>,
    response_channel: Channel<CriticalSectionRawMutex, Result<(), LcdTextError>, 8>,
}

impl LcdTextStatic {
    /// Creates static resources for the character LCD device.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            request_channel: Channel::new(),
            response_channel: Channel::new(),
        }
    }

    /// Queue one message to the LCD task.
    pub async fn send_request(&self, message: LcdTextMessage) {
        self.request_channel.send(message).await;
    }

    /// Wait for the next message for the LCD task.
    pub async fn receive_request(&self) -> LcdTextMessage {
        self.request_channel.receive().await
    }

    /// Queue one operation result from the LCD task.
    pub async fn send_response(&self, response: Result<(), LcdTextError>) {
        self.response_channel.send(response).await;
    }

    /// Wait for the next operation result from the LCD task.
    pub async fn receive_response(&self) -> Result<(), LcdTextError> {
        self.response_channel.receive().await
    }
}

/// Character LCD write adapter for platform crates.
pub trait LcdTextWrite {
    /// Write one byte to the configured LCD I2C expander.
    fn write(&mut self, address: u8, data: u8) -> Result<(), LcdTextError>;
}

// PCF8574 pin mapping: P0=RS, P1=RW, P2=E, P3=Backlight, P4-P7=Data.
const LCD_BACKLIGHT: u8 = 0x08;
const LCD_ENABLE: u8 = 0x04;
const LCD_RS: u8 = 0x01;

/// HD44780 protocol driver over a byte-oriented I2C expander transport.
pub struct LcdTextDriver {
    address: u8,
}

impl LcdTextDriver {
    /// Creates a driver for the default PCF8574 backpack address (`0x27`).
    #[must_use]
    pub const fn new_default() -> Self {
        Self { address: 0x27 }
    }

    /// Initialize the LCD in 4-bit mode and clear it.
    pub async fn init(&mut self, lcd_text_write: &mut impl LcdTextWrite) -> Result<(), LcdTextError> {
        Timer::after_millis(50).await;

        self.write_nibble(lcd_text_write, 0x03, false).await?;
        Timer::after_millis(5).await;
        self.write_nibble(lcd_text_write, 0x03, false).await?;
        Timer::after_micros(150).await;
        self.write_nibble(lcd_text_write, 0x03, false).await?;
        self.write_nibble(lcd_text_write, 0x02, false).await?;

        // Function set: 4-bit, 2 lines, 5x8 font.
        self.write_byte(lcd_text_write, 0x28, false).await?;
        // Display control: display on, cursor off, blink off.
        self.write_byte(lcd_text_write, 0x0C, false).await?;
        // Clear display.
        self.write_byte(lcd_text_write, 0x01, false).await?;
        Timer::after_millis(2).await;
        // Entry mode: increment cursor, no shift.
        self.write_byte(lcd_text_write, 0x06, false).await?;
        Ok(())
    }

    /// Process one queued LCD message.
    pub async fn process_message(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        lcd_text_message: LcdTextMessage,
    ) -> Result<(), LcdTextError> {
        match lcd_text_message {
            LcdTextMessage::Display { text, duration_ms } => {
                self.clear(lcd_text_write).await?;
                if let Some((line1, line2)) = text.as_str().split_once('\n') {
                    self.print(lcd_text_write, line1).await?;
                    self.set_cursor(lcd_text_write, 1, 0).await?;
                    self.print(lcd_text_write, line2).await?;
                } else {
                    self.print(lcd_text_write, text.as_str()).await?;
                }

                if duration_ms > 0 {
                    Timer::after_millis(duration_ms.into()).await;
                }
            }
        }
        Ok(())
    }

    #[expect(clippy::arithmetic_side_effects, reason = "Bit operations")]
    async fn write_nibble(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        nibble: u8,
        rs: bool,
    ) -> Result<(), LcdTextError> {
        let rs_bit = if rs { LCD_RS } else { 0 };
        let data = (nibble << 4) | LCD_BACKLIGHT | rs_bit;

        lcd_text_write.write(self.address, data | LCD_ENABLE)?;
        Timer::after_micros(1).await;
        lcd_text_write.write(self.address, data)?;
        Timer::after_micros(50).await;
        Ok(())
    }

    async fn write_byte(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        byte: u8,
        rs: bool,
    ) -> Result<(), LcdTextError> {
        self.write_nibble(lcd_text_write, (byte >> 4) & 0x0F, rs)
            .await?;
        self.write_nibble(lcd_text_write, byte & 0x0F, rs).await?;
        Ok(())
    }

    async fn clear(&mut self, lcd_text_write: &mut impl LcdTextWrite) -> Result<(), LcdTextError> {
        self.write_byte(lcd_text_write, 0x01, false).await?;
        Timer::after_millis(2).await;
        Ok(())
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "Row/column values are small"
    )]
    async fn set_cursor(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        row: u8,
        col: u8,
    ) -> Result<(), LcdTextError> {
        let address = match row {
            0 => col,
            1 => 0x40 + col,
            2 => 0x14 + col,
            3 => 0x54 + col,
            _ => 0,
        };
        self.write_byte(lcd_text_write, 0x80 | address, false).await?;
        Ok(())
    }

    async fn print(&mut self, lcd_text_write: &mut impl LcdTextWrite, text: &str) -> Result<(), LcdTextError> {
        for ch in text.bytes() {
            self.write_byte(lcd_text_write, ch, true).await?;
        }
        Ok(())
    }
}
