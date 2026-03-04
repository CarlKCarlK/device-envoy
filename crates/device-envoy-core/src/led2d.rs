//! Shared 2D LED panel building blocks used across all device-envoy platforms.
//!
//! This module provides platform-independent types for NeoPixel-style (WS2812) LED panel
//! displays. See the platform crate (`device-envoy-rp` or `device-envoy-esp`) for the
//! primary documentation and examples.

pub mod layout;

pub use embedded_graphics::geometry::Point;
pub use embedded_graphics::geometry::Size;
pub use layout::LedLayout;

use core::{
    convert::Infallible,
    ops::{Deref, DerefMut, Index, IndexMut},
};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{
        DecorationDimensions, MonoFont,
        ascii::{
            FONT_4X6, FONT_5X7, FONT_5X8, FONT_6X9, FONT_6X10, FONT_6X12, FONT_6X13,
            FONT_6X13_BOLD, FONT_6X13_ITALIC, FONT_7X13, FONT_7X13_BOLD, FONT_7X13_ITALIC,
            FONT_7X14, FONT_7X14_BOLD, FONT_8X13, FONT_8X13_BOLD, FONT_8X13_ITALIC, FONT_9X15,
            FONT_9X15_BOLD, FONT_9X18, FONT_9X18_BOLD, FONT_10X20,
        },
        mapping::StrGlyphMapping,
    },
    prelude::*,
};
use smart_leds::RGB8;

use crate::led_strip::ToRgb888;

/// Platform-agnostic LED panel device contract.
///
/// Platform crates implement this for their concrete LED panel types so shared logic can
/// write 2D frames without knowing the underlying hardware backend.
pub trait Led2dDevice<const W: usize, const H: usize> {
    /// Write a frame to the LED panel.
    fn write_frame2d(&mut self, frame2d: &Frame2d<W, H>);
}

// Packed bitmap for the internal 3x4 font (ASCII 0x20-0x7E).
const BIT_MATRIX3X4_FONT_DATA: [u8; 144] = [
    0x0a, 0xd5, 0x10, 0x4a, 0xa0, 0x01, 0x0a, 0xfe, 0x68, 0x85, 0x70, 0x02, 0x08, 0x74, 0x90, 0x86,
    0xa5, 0xc4, 0x08, 0x5e, 0x68, 0x48, 0x08, 0x10, 0xeb, 0x7b, 0xe7, 0xfd, 0x22, 0x27, 0xb8, 0x9b,
    0x39, 0xb4, 0x05, 0xd1, 0xa9, 0x3e, 0xea, 0x5d, 0x28, 0x0a, 0xff, 0xf3, 0xfc, 0xe4, 0x45, 0xd2,
    0xff, 0x7d, 0xff, 0xbc, 0xd9, 0xff, 0xb7, 0xcb, 0xb4, 0xe8, 0xe9, 0xfd, 0xfe, 0xcb, 0x25, 0xaa,
    0xd9, 0x7d, 0x97, 0x7d, 0xe7, 0xbf, 0xdf, 0x6f, 0xdf, 0x7f, 0x6d, 0xb7, 0xe0, 0xd0, 0xf7, 0xe5,
    0x6d, 0x48, 0xc0, 0x68, 0xdf, 0x35, 0x6f, 0x49, 0x40, 0x40, 0x86, 0xf5, 0xd7, 0xab, 0xe0, 0xc7,
    0x5f, 0x7d, 0xff, 0xbc, 0xd9, 0xff, 0x37, 0xcb, 0xb4, 0xe8, 0xe9, 0xfd, 0x1e, 0xcb, 0x25, 0xaa,
    0xd9, 0x7d, 0x17, 0x7d, 0xe7, 0xbf, 0xdf, 0x6f, 0xdf, 0x7f, 0x6d, 0xb7, 0xb1, 0x80, 0xf7, 0xe5,
    0x6d, 0x48, 0xa0, 0xa8, 0xdf, 0x35, 0x6f, 0x49, 0x20, 0x90, 0x86, 0xf5, 0xd7, 0xab, 0xb1, 0x80,
];
const BIT_MATRIX3X4_IMAGE_WIDTH: u32 = 48;
const BIT_MATRIX3X4_GLYPH_MAPPING: StrGlyphMapping<'static> = StrGlyphMapping::new("\0 \u{7e}", 0);

/// Monospace 3x4 font matching the internal `BIT_MATRIX3X4` bitmap data.
#[must_use]
pub fn bit_matrix3x4_font() -> MonoFont<'static> {
    MonoFont {
        image: embedded_graphics::image::ImageRaw::new(
            &BIT_MATRIX3X4_FONT_DATA,
            BIT_MATRIX3X4_IMAGE_WIDTH,
        ),
        glyph_mapping: &BIT_MATRIX3X4_GLYPH_MAPPING,
        character_size: embedded_graphics::prelude::Size::new(3, 4),
        character_spacing: 0,
        baseline: 3,
        underline: DecorationDimensions::new(3, 1),
        strikethrough: DecorationDimensions::new(2, 1),
    }
}

/// Render text into a frame using the provided font.
///
/// Text flows left-to-right within the frame width; a `\n` character advances to the next row.
/// Characters that exceed the frame width are skipped (no wrapping). Colors cycle over the
/// `colors` slice (one color per character); an empty slice defaults to white.
///
/// `spacing_reduction` is a `(width_reduction, height_reduction)` pair in pixels used by the
/// trimmed [`Led2dFont`] variants to pack characters more tightly.
pub fn render_text_to_frame<const W: usize, const H: usize>(
    frame: &mut Frame2d<W, H>,
    font: &embedded_graphics::mono_font::MonoFont<'static>,
    text: &str,
    colors: &[RGB8],
    spacing_reduction: (i32, i32),
) {
    let glyph_width = font.character_size.width as i32;
    let glyph_height = font.character_size.height as i32;
    let advance_x = glyph_width - spacing_reduction.0;
    let advance_y = glyph_height - spacing_reduction.1;
    let width_limit = W as i32;
    let height_limit = H as i32;
    if height_limit <= 0 || width_limit <= 0 {
        return;
    }
    let baseline = font.baseline as i32;
    let mut x = 0i32;
    let mut y = baseline;
    let mut color_index: usize = 0;

    for ch in text.chars() {
        if ch == '\n' {
            x = 0;
            y += advance_y;
            if y - baseline >= height_limit {
                break;
            }
            continue;
        }

        // Clip characters that exceed width limit (no wrapping until explicit \n).
        if x + advance_x > width_limit {
            continue;
        }

        let color = if colors.is_empty() {
            smart_leds::colors::WHITE
        } else {
            colors[color_index % colors.len()]
        };
        color_index = color_index.wrapping_add(1);

        let mut buf = [0u8; 4];
        let slice = ch.encode_utf8(&mut buf);
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, color.to_rgb888());
        let position = embedded_graphics::prelude::Point::new(x, y);
        embedded_graphics::Drawable::draw(
            &embedded_graphics::text::Text::new(slice, position, style),
            frame,
        )
        .expect("drawing into frame cannot fail");

        x += advance_x;
    }
}

/// Fonts available for use with LED panel displays.
///
/// Fonts with `Trim` suffix remove blank spacing to pack text more tightly on small displays.
#[derive(Clone, Copy, Debug)]
pub enum Led2dFont {
    /// 3x4 monospace font, trimmed (compact layout).
    Font3x4Trim,
    /// 4x6 monospace font.
    Font4x6,
    /// 3x5 monospace font, trimmed (compact layout).
    Font3x5Trim,
    /// 5x7 monospace font.
    Font5x7,
    /// 4x6 monospace font, trimmed (compact layout).
    Font4x6Trim,
    /// 5x8 monospace font.
    Font5x8,
    /// 4x7 monospace font, trimmed (compact layout).
    Font4x7Trim,
    /// 6x9 monospace font.
    Font6x9,
    /// 5x8 monospace font, trimmed (compact layout).
    Font5x8Trim,
    /// 6x10 monospace font.
    Font6x10,
    /// 5x9 monospace font, trimmed (compact layout).
    Font5x9Trim,
    /// 6x12 monospace font.
    Font6x12,
    /// 5x11 monospace font, trimmed (compact layout).
    Font5x11Trim,
    /// 6x13 monospace font.
    Font6x13,
    /// 5x12 monospace font, trimmed (compact layout).
    Font5x12Trim,
    /// 6x13 bold monospace font.
    Font6x13Bold,
    /// 5x12 bold monospace font, trimmed (compact layout).
    Font5x12TrimBold,
    /// 6x13 italic monospace font.
    Font6x13Italic,
    /// 5x12 italic monospace font, trimmed (compact layout).
    Font5x12TrimItalic,
    /// 7x13 monospace font.
    Font7x13,
    /// 6x12 monospace font, trimmed (compact layout).
    Font6x12Trim,
    /// 7x13 bold monospace font.
    Font7x13Bold,
    /// 6x12 bold monospace font, trimmed (compact layout).
    Font6x12TrimBold,
    /// 7x13 italic monospace font.
    Font7x13Italic,
    /// 6x12 italic monospace font, trimmed (compact layout).
    Font6x12TrimItalic,
    /// 7x14 monospace font.
    Font7x14,
    /// 6x13 monospace font, trimmed (compact layout).
    Font6x13Trim,
    /// 7x14 bold monospace font.
    Font7x14Bold,
    /// 6x13 bold monospace font, trimmed (compact layout).
    Font6x13TrimBold,
    /// 8x13 monospace font.
    Font8x13,
    /// 7x12 monospace font, trimmed (compact layout).
    Font7x12Trim,
    /// 8x13 bold monospace font.
    Font8x13Bold,
    /// 7x12 bold monospace font, trimmed (compact layout).
    Font7x12TrimBold,
    /// 8x13 italic monospace font.
    Font8x13Italic,
    /// 7x12 italic monospace font, trimmed (compact layout).
    Font7x12TrimItalic,
    /// 9x15 monospace font.
    Font9x15,
    /// 8x14 monospace font, trimmed (compact layout).
    Font8x14Trim,
    /// 9x15 bold monospace font.
    Font9x15Bold,
    /// 8x14 bold monospace font, trimmed (compact layout).
    Font8x14TrimBold,
    /// 9x18 monospace font.
    Font9x18,
    /// 8x17 monospace font, trimmed (compact layout).
    Font8x17Trim,
    /// 9x18 bold monospace font.
    Font9x18Bold,
    /// 8x17 bold monospace font, trimmed (compact layout).
    Font8x17TrimBold,
    /// 10x20 monospace font.
    Font10x20,
    /// 9x19 monospace font, trimmed (compact layout).
    Font9x19Trim,
}

impl Led2dFont {
    /// Return the `MonoFont` for this variant.
    #[must_use]
    pub fn to_font(self) -> MonoFont<'static> {
        match self {
            Self::Font3x4Trim => bit_matrix3x4_font(),
            Self::Font4x6 | Self::Font3x5Trim => FONT_4X6,
            Self::Font5x7 | Self::Font4x6Trim => FONT_5X7,
            Self::Font5x8 | Self::Font4x7Trim => FONT_5X8,
            Self::Font6x9 | Self::Font5x8Trim => FONT_6X9,
            Self::Font6x10 | Self::Font5x9Trim => FONT_6X10,
            Self::Font6x12 | Self::Font5x11Trim => FONT_6X12,
            Self::Font6x13 | Self::Font5x12Trim => FONT_6X13,
            Self::Font6x13Bold | Self::Font5x12TrimBold => FONT_6X13_BOLD,
            Self::Font6x13Italic | Self::Font5x12TrimItalic => FONT_6X13_ITALIC,
            Self::Font7x13 | Self::Font6x12Trim => FONT_7X13,
            Self::Font7x13Bold | Self::Font6x12TrimBold => FONT_7X13_BOLD,
            Self::Font7x13Italic | Self::Font6x12TrimItalic => FONT_7X13_ITALIC,
            Self::Font7x14 | Self::Font6x13Trim => FONT_7X14,
            Self::Font7x14Bold | Self::Font6x13TrimBold => FONT_7X14_BOLD,
            Self::Font8x13 | Self::Font7x12Trim => FONT_8X13,
            Self::Font8x13Bold | Self::Font7x12TrimBold => FONT_8X13_BOLD,
            Self::Font8x13Italic | Self::Font7x12TrimItalic => FONT_8X13_ITALIC,
            Self::Font9x15 | Self::Font8x14Trim => FONT_9X15,
            Self::Font9x15Bold | Self::Font8x14TrimBold => FONT_9X15_BOLD,
            Self::Font9x18 | Self::Font8x17Trim => FONT_9X18,
            Self::Font9x18Bold | Self::Font8x17TrimBold => FONT_9X18_BOLD,
            Self::Font10x20 | Self::Font9x19Trim => FONT_10X20,
        }
    }

    /// Return spacing reduction for trimmed variants as `(width_reduction, height_reduction)`.
    #[must_use]
    pub const fn spacing_reduction(self) -> (i32, i32) {
        match self {
            Self::Font3x4Trim
            | Self::Font4x6
            | Self::Font5x7
            | Self::Font5x8
            | Self::Font6x9
            | Self::Font6x10
            | Self::Font6x12
            | Self::Font6x13
            | Self::Font6x13Bold
            | Self::Font6x13Italic
            | Self::Font7x13
            | Self::Font7x13Bold
            | Self::Font7x13Italic
            | Self::Font7x14
            | Self::Font7x14Bold
            | Self::Font8x13
            | Self::Font8x13Bold
            | Self::Font8x13Italic
            | Self::Font9x15
            | Self::Font9x15Bold
            | Self::Font9x18
            | Self::Font9x18Bold
            | Self::Font10x20 => (0, 0),
            Self::Font3x5Trim
            | Self::Font4x6Trim
            | Self::Font4x7Trim
            | Self::Font5x8Trim
            | Self::Font5x9Trim
            | Self::Font5x11Trim
            | Self::Font5x12Trim
            | Self::Font5x12TrimBold
            | Self::Font5x12TrimItalic
            | Self::Font6x12Trim
            | Self::Font6x12TrimBold
            | Self::Font6x12TrimItalic
            | Self::Font6x13Trim
            | Self::Font6x13TrimBold
            | Self::Font7x12Trim
            | Self::Font7x12TrimBold
            | Self::Font7x12TrimItalic
            | Self::Font8x14Trim
            | Self::Font8x14TrimBold
            | Self::Font8x17Trim
            | Self::Font8x17TrimBold
            | Self::Font9x19Trim => (1, 1),
        }
    }
}

/// 2D pixel array used for general graphics on LED panels.
///
/// - Coordinates are `(x, y)` with `(0, 0)` at the top-left. The x-axis increases to the
///   right, and the y-axis increases downward.
/// - Set pixels using tuple indexing: `frame[(x, y)] = colors::RED;`.
/// - For shapes, lines, and text rendering, use the [`embedded-graphics`](https://docs.rs/embedded-graphics) crate.
///
/// ## Indexing and storage
///
/// `Frame2d` supports both:
///
/// - `(x, y)` tuple indexing: `frame[(x, y)]`
/// - Row-major array indexing: `frame[y][x]`
///
/// Tuple indexing matches display coordinates. Array indexing matches the underlying storage.
///
/// # Example: Draw pixels both directly and with [`embedded-graphics`](https://docs.rs/embedded-graphics)
///
/// ```rust,no_run
/// use device_envoy_core::{led2d::Frame2d, led_strip::ToRgb888};
/// use embedded_graphics::{
///     prelude::*,
///     primitives::{Circle, PrimitiveStyle, Rectangle},
/// };
/// use smart_leds::colors;
/// # fn example() {
///
/// type Frame = Frame2d<12, 8>;
///
/// /// Calculate the top-left corner position to center a shape within a bounding box.
/// const fn centered_top_left(width: usize, height: usize, size: usize) -> Point {
///     assert!(size <= width);
///     assert!(size <= height);
///     Point::new(((width - size) / 2) as i32, ((height - size) / 2) as i32)
/// }
///
/// // Create a frame to draw on. This is just an in-memory 2D pixel buffer.
/// let mut frame = Frame::new();
///
/// // Use the embedded-graphics crate to draw a red rectangle border around the edge of the frame.
/// // We use `to_rgb888()` to convert from smart-leds RGB8 to embedded-graphics Rgb888.
/// Rectangle::new(Frame::TOP_LEFT, Frame::SIZE)
///     .into_styled(PrimitiveStyle::with_stroke(colors::RED.to_rgb888(), 1))
///     .draw(&mut frame)
///     .expect("rectangle draw must succeed");
///
/// // Direct pixel access: set the upper-left LED pixel (x = 0, y = 0).
/// // Frame2d stores LED colors directly, so we write an LED color here.
/// frame[(0, 0)] = colors::CYAN;
///
/// // Use the embedded-graphics crate to draw a green circle centered in the frame.
/// const DIAMETER: u32 = 6;
/// const CIRCLE_TOP_LEFT: Point = centered_top_left(Frame::WIDTH, Frame::HEIGHT, DIAMETER as usize);
/// Circle::new(CIRCLE_TOP_LEFT, DIAMETER)
///     .into_styled(PrimitiveStyle::with_stroke(colors::LIME.to_rgb888(), 1))
///     .draw(&mut frame)
///     .expect("circle draw must succeed");
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Frame2d<const W: usize, const H: usize>(pub [[RGB8; W]; H]);

impl<const W: usize, const H: usize> Frame2d<W, H> {
    /// The width of the frame.
    pub const WIDTH: usize = W;
    /// The height of the frame.
    pub const HEIGHT: usize = H;
    /// Total pixels in this frame (width × height).
    pub const LEN: usize = W * H;
    /// Frame dimensions as a [`Size`].
    ///
    /// For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations.
    pub const SIZE: Size = Size::new(W as u32, H as u32);
    /// Top-left corner coordinate as a [`Point`].
    ///
    /// For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations.
    pub const TOP_LEFT: Point = Point::new(0, 0);
    /// Top-right corner coordinate as a [`Point`].
    ///
    /// For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations.
    pub const TOP_RIGHT: Point = Point::new((W - 1) as i32, 0);
    /// Bottom-left corner coordinate as a [`Point`].
    ///
    /// For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations.
    pub const BOTTOM_LEFT: Point = Point::new(0, (H - 1) as i32);
    /// Bottom-right corner coordinate as a [`Point`].
    ///
    /// For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations.
    pub const BOTTOM_RIGHT: Point = Point::new((W - 1) as i32, (H - 1) as i32);

    /// Create a new blank (all black) frame.
    #[must_use]
    pub const fn new() -> Self {
        Self([[RGB8::new(0, 0, 0); W]; H])
    }

    /// Create a frame filled with a single color.
    #[must_use]
    pub const fn filled(color: RGB8) -> Self {
        Self([[color; W]; H])
    }
}

impl<const W: usize, const H: usize> Deref for Frame2d<W, H> {
    type Target = [[RGB8; W]; H];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const W: usize, const H: usize> DerefMut for Frame2d<W, H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const W: usize, const H: usize> Index<(usize, usize)> for Frame2d<W, H> {
    type Output = RGB8;

    fn index(&self, (x_index, y_index): (usize, usize)) -> &Self::Output {
        assert!(x_index < W, "x_index must be within width");
        assert!(y_index < H, "y_index must be within height");
        &self.0[y_index][x_index]
    }
}

impl<const W: usize, const H: usize> IndexMut<(usize, usize)> for Frame2d<W, H> {
    fn index_mut(&mut self, (x_index, y_index): (usize, usize)) -> &mut Self::Output {
        assert!(x_index < W, "x_index must be within width");
        assert!(y_index < H, "y_index must be within height");
        &mut self.0[y_index][x_index]
    }
}

impl<const W: usize, const H: usize> From<[[RGB8; W]; H]> for Frame2d<W, H> {
    fn from(array: [[RGB8; W]; H]) -> Self {
        Self(array)
    }
}

impl<const W: usize, const H: usize> From<Frame2d<W, H>> for [[RGB8; W]; H] {
    fn from(frame: Frame2d<W, H>) -> Self {
        frame.0
    }
}

impl<const W: usize, const H: usize> Default for Frame2d<W, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize> OriginDimensions for Frame2d<W, H> {
    fn size(&self) -> Size {
        Size::new(W as u32, H as u32)
    }
}

impl<const W: usize, const H: usize> DrawTarget for Frame2d<W, H> {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            let x_index = coord.x;
            let y_index = coord.y;
            if x_index >= 0 && x_index < W as i32 && y_index >= 0 && y_index < H as i32 {
                self.0[y_index as usize][x_index as usize] =
                    RGB8::new(color.r(), color.g(), color.b());
            }
        }
        Ok(())
    }
}
