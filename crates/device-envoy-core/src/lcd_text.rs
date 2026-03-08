//! A device abstraction for shared HD44780 character LCD protocol/state helpers.
//!
//! See `device_envoy_rp::lcd_text` for constructors and usage examples.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

/// Maximum characters supported by this shared frame container (20x4).
pub const MAX_LCD_CHARS: usize = 80;

/// Character LCD operation errors shared across platform crates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcdTextError {
    /// I2C write failed for the given 7-bit address.
    I2cWrite { address: u8 },
    /// Attempted to set cursor to an out-of-range row.
    RowOutOfBounds { row: usize },
}

/// A packed text frame for an HD44780 display.
#[derive(Clone, Copy, Debug)]
pub struct LcdTextFrame {
    /// Frame width in characters.
    pub width: usize,
    /// Frame height in characters.
    pub height: usize,
    /// Packed row-major cell bytes.
    pub cells: [u8; MAX_LCD_CHARS],
}

impl LcdTextFrame {
    /// Create a blank frame with spaces.
    #[must_use]
    pub const fn new_blank(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: [b' '; MAX_LCD_CHARS],
        }
    }

    /// Build a packed frame from a fixed `W x H` buffer.
    #[must_use]
    pub fn from_rows<const W: usize, const H: usize>(rows: [[u8; W]; H]) -> Self {
        let mut lcd_text_frame = Self::new_blank(W, H);
        let mut row_index = 0;
        while row_index < H {
            let mut column_index = 0;
            while column_index < W {
                let flat_index = row_index * W + column_index;
                lcd_text_frame.cells[flat_index] = rows[row_index][column_index];
                column_index += 1;
            }
            row_index += 1;
        }
        lcd_text_frame
    }

    /// Returns the byte at `(row, col)` in this frame.
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> u8 {
        self.cells[row * self.width + col]
    }
}

/// Static signal resources for LCD frame delivery.
pub struct LcdTextStatic {
    frame_signal: Signal<CriticalSectionRawMutex, LcdTextFrame>,
}

impl LcdTextStatic {
    /// Creates static resources for the character LCD device.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            frame_signal: Signal::new(),
        }
    }

    /// Signal one full frame to the LCD task.
    pub fn signal_frame(&self, frame: LcdTextFrame) {
        self.frame_signal.signal(frame);
    }

    /// Wait for the next frame signaled to the LCD task.
    pub async fn wait_frame(&self) -> LcdTextFrame {
        self.frame_signal.wait().await
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
    /// Creates a driver for a specific PCF8574 backpack address.
    #[must_use]
    pub const fn new(address: u8) -> Self {
        Self { address }
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

    /// Write one full frame to the LCD.
    pub async fn write_frame(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        lcd_text_frame: &LcdTextFrame,
    ) -> Result<(), LcdTextError> {
        self.clear(lcd_text_write).await?;

        for row_index in 0..lcd_text_frame.height {
            self.set_cursor(lcd_text_write, row_index, 0).await?;
            for column_index in 0..lcd_text_frame.width {
                self.write_byte(lcd_text_write, lcd_text_frame.cell(row_index, column_index), true)
                    .await?;
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
        row: usize,
        col: u8,
    ) -> Result<(), LcdTextError> {
        let address = match row {
            0 => col,
            1 => 0x40_u8 + col,
            2 => 0x14_u8 + col,
            3 => 0x54_u8 + col,
            _ => return Err(LcdTextError::RowOutOfBounds { row }),
        };
        self.write_byte(lcd_text_write, 0x80 | address, false).await?;
        Ok(())
    }
}
