//! A device abstraction for shared HD44780 character LCD protocol/state helpers.
//!
//! See `device_envoy_rp::lcd_text` for constructors and usage examples.

use embassy_time::Timer;

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
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
pub struct LcdTextFrame<const MAX_CHARS: usize> {
    /// Frame width in characters.
    pub width: usize,
    /// Frame height in characters.
    pub height: usize,
    /// Packed row-major cell bytes.
    pub cells: [u8; MAX_CHARS],
}

impl<const MAX_CHARS: usize> LcdTextFrame<MAX_CHARS> {
    /// Create a blank frame with spaces.
    #[must_use]
    pub const fn new_blank(width: usize, height: usize) -> Self {
        assert!(
            width * height <= MAX_CHARS,
            "frame geometry exceeds capacity"
        );
        Self {
            width,
            height,
            cells: [b' '; MAX_CHARS],
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

/// Render text into a fixed-size LCD frame using `W x H` geometry.
///
/// Behavior:
/// - `\n` starts a new row.
/// - Characters past `W` on a row are ignored.
/// - Rows past `H` are ignored.
/// - Non-ASCII Unicode characters are replaced with `?`.
/// - Missing characters are padded with spaces.
#[must_use]
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
pub fn render_lcd_text_frame<const W: usize, const H: usize, const MAX_CHARS: usize>(
    text: &str,
) -> LcdTextFrame<MAX_CHARS> {
    let mut rows = [[b' '; W]; H];

    for (row_index, line) in text.split('\n').enumerate() {
        if row_index >= H {
            break;
        }

        for (column_index, ch) in line.chars().enumerate() {
            if column_index >= W {
                break;
            }
            rows[row_index][column_index] = if ch.is_ascii() { ch as u8 } else { b'?' };
        }
    }

    LcdTextFrame::<MAX_CHARS>::from_rows(rows)
}

/// Platform-agnostic LCD text device contract.
///
/// Platform crates implement this trait for their generated LCD text types so
/// shared logic can write text without knowing the hardware backend.
///
/// Design intent:
///
/// - This trait is intended for static dispatch on embedded targets.
/// - Dimensions are const generics so geometry remains compile-time.
/// - `write_text` accepts any string-like input via `AsRef<str>`.
///
/// # Example: Write Text
///
/// This example writes text through a generic trait-bound helper.
///
/// ```rust,no_run
/// use device_envoy_core::lcd_text::LcdText;
///
/// fn write_message<const W: usize, const H: usize>(lcd_text: &impl LcdText<W, H>) {
///     lcd_text.write_text("Hello from\ndevice-envoy!");
/// }
///
/// # struct LcdTextSimple;
/// # impl LcdText<16, 2> for LcdTextSimple {
/// #     const ADDRESS: u8 = 0x27;
/// #     fn write_text(&self, _text: impl AsRef<str>) {}
/// # }
/// # let lcd_text_simple = LcdTextSimple;
/// # write_message(&lcd_text_simple);
/// ```
pub trait LcdText<const W: usize, const H: usize> {
    /// Display width in characters.
    const WIDTH: usize = W;
    /// Display height in characters.
    const HEIGHT: usize = H;
    /// LCD I2C address.
    const ADDRESS: u8;

    /// Write text to the display.
    /// See the [LcdText trait documentation](Self) for usage examples.
    fn write_text(&self, text: impl AsRef<str>);
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
// Public for cross-crate platform plumbing; hidden from end-user docs.
#[doc(hidden)]
pub struct LcdTextDriver {
    address: u8,
}

impl LcdTextDriver {
    /// Creates a driver for a specific PCF8574 backpack address.
    #[must_use]
    pub const fn new(address: u8) -> Self {
        Self { address }
    }

    /// Set the active LCD I2C address for subsequent writes.
    pub fn set_address(&mut self, address: u8) {
        self.address = address;
    }

    /// Initialize the LCD in 4-bit mode and clear it.
    pub async fn init(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
    ) -> Result<(), LcdTextError> {
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
    pub async fn write_frame<const MAX_CHARS: usize>(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        lcd_text_frame: &LcdTextFrame<MAX_CHARS>,
    ) -> Result<(), LcdTextError> {
        self.clear(lcd_text_write).await?;

        for row_index in 0..lcd_text_frame.height {
            self.set_cursor(lcd_text_write, row_index, 0).await?;
            for column_index in 0..lcd_text_frame.width {
                self.write_byte(
                    lcd_text_write,
                    lcd_text_frame.cell(row_index, column_index),
                    true,
                )
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
        self.write_byte(lcd_text_write, 0x80 | address, false)
            .await?;
        Ok(())
    }
}
