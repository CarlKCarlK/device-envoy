//! A device abstraction for shared HD44780 character LCD protocol/state helpers.
//!
//! See `device_envoy_rp::lcd_text` for constructors and usage examples.

use embassy_time::Timer;

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
    fn write(&mut self, address: u8, data: u8) -> crate::Result<()>;
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
    pub async fn init(&mut self, lcd_text_write: &mut impl LcdTextWrite) -> crate::Result<()> {
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
    ) -> crate::Result<()> {
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

    async fn write_nibble(
        &mut self,
        lcd_text_write: &mut impl LcdTextWrite,
        nibble: u8,
        rs: bool,
    ) -> crate::Result<()> {
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
    ) -> crate::Result<()> {
        self.write_nibble(lcd_text_write, (byte >> 4) & 0x0F, rs)
            .await?;
        self.write_nibble(lcd_text_write, byte & 0x0F, rs).await?;
        Ok(())
    }

    async fn clear(&mut self, lcd_text_write: &mut impl LcdTextWrite) -> crate::Result<()> {
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
    ) -> crate::Result<()> {
        let address = match row {
            0 => col,
            1 => 0x40_u8 + col,
            2 => 0x14_u8 + col,
            3 => 0x54_u8 + col,
            _ => return Err(crate::Error::LcdRowOutOfBounds { row }),
        };
        self.write_byte(lcd_text_write, 0x80 | address, false)
            .await?;
        Ok(())
    }
}

// ============================================================================
// Shared runtime for the `i2cs!` and `lcd_text!` macros on every chip.
// Public because the macros expand in downstream crates; hidden from user docs.
// ============================================================================

/// Carries one display's next frame to its group's I2C task.
#[doc(hidden)]
pub type __I2csSignal<T> =
    embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, T>;

#[doc(hidden)]
pub use LcdText as __LcdText;
#[doc(hidden)]
pub use LcdTextFrame as __LcdTextFrame;

/// The largest `width * height` among a group's displays: the frame size every member's
/// signal carries.
#[doc(hidden)]
#[must_use]
pub const fn __max_lcd_cells<const N: usize>(widths: [usize; N], heights: [usize; N]) -> usize {
    let mut max_cells = 0;
    let mut index = 0;
    while index < N {
        let cells = widths[index] * heights[index];
        if cells > max_cells {
            max_cells = cells;
        }
        index += 1;
    }
    max_cells
}

/// Fails to compile when two displays in one group share an I2C address.
#[doc(hidden)]
pub const fn __assert_unique_addresses<const N: usize>(addresses: [u8; N]) {
    let mut first_index = 0;
    while first_index < N {
        let mut second_index = first_index + 1;
        while second_index < N {
            if addresses[first_index] == addresses[second_index] {
                panic!("duplicate lcd_text I2C address in i2cs! group");
            }
            second_index += 1;
        }
        first_index += 1;
    }
}

/// Renders `text` for a `W` by `H` display and hands it to the display's group task.
#[doc(hidden)]
pub fn __write_text<const W: usize, const H: usize, const MAX_CHARS: usize>(
    signal: &__I2csSignal<LcdTextFrame<MAX_CHARS>>,
    text: &str,
) {
    assert!(W > 0, "lcd_text width must be > 0");
    assert!(H > 0, "lcd_text height must be > 0");
    assert!(H <= 4, "lcd_text height must be <= 4 for HD44780 row map");
    signal.signal(render_lcd_text_frame::<W, H, MAX_CHARS>(text));
}

/// Runs a group's shared I2C task: waits for any display's next frame and writes it,
/// initializing each display the first time it is written. A failed write is passed to
/// `report_error`, which each chip crate supplies so it can log in its own way, and the
/// task continues with the next frame.
#[doc(hidden)]
pub async fn __run_i2cs<const N: usize, const MAX_CHARS: usize>(
    mut lcd_text_write: impl LcdTextWrite,
    signals: [&'static __I2csSignal<LcdTextFrame<MAX_CHARS>>; N],
    addresses: [u8; N],
    widths: [usize; N],
    heights: [usize; N],
    report_error: fn(crate::Error),
) -> ! {
    let mut lcd_text_driver = LcdTextDriver::new(0x27);
    let mut initialized = [false; N];
    loop {
        let (lcd_text_frame, ready_index) =
            embassy_futures::select::select_array(signals.map(|signal| signal.wait())).await;
        if let Err(error) = write_lcd_text_cells::<MAX_CHARS>(
            &mut lcd_text_driver,
            &mut lcd_text_write,
            &mut initialized[ready_index],
            addresses[ready_index],
            widths[ready_index],
            heights[ready_index],
            &lcd_text_frame.cells,
        )
        .await
        {
            report_error(error);
        }
    }
}

/// Writes one frame to the display at `address`, initializing it first if it has not
/// been. A display whose initialization fails is initialized again on its next frame.
async fn write_lcd_text_cells<const MAX_CHARS: usize>(
    lcd_text_driver: &mut LcdTextDriver,
    lcd_text_write: &mut impl LcdTextWrite,
    initialized: &mut bool,
    address: u8,
    width: usize,
    height: usize,
    cells: &[u8],
) -> crate::Result<()> {
    lcd_text_driver.set_address(address);
    if !*initialized {
        lcd_text_driver.init(lcd_text_write).await?;
        *initialized = true;
    }

    let mut lcd_text_frame = LcdTextFrame::<MAX_CHARS>::new_blank(width, height);
    let cell_count = core::cmp::min(width * height, cells.len());
    for cell_index in 0..cell_count {
        lcd_text_frame.cells[cell_index] = cells[cell_index];
    }

    lcd_text_driver
        .write_frame(lcd_text_write, &lcd_text_frame)
        .await
}
