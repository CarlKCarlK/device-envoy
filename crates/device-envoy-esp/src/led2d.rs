//! A device abstraction for rectangular NeoPixel-style (WS2812) LED panel displays.
//! See [`Led2d`] for the runtime adapter and [`layout::LedLayout`] for compile-time
//! panel wiring and geometry.

pub mod layout;

use core::{
    borrow::Borrow,
    convert::Infallible,
    ops::{Deref, DerefMut, Index, IndexMut},
};

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{
        ascii::{
            FONT_10X20, FONT_4X6, FONT_5X7, FONT_5X8, FONT_6X10, FONT_6X12, FONT_6X13,
            FONT_6X13_BOLD, FONT_6X13_ITALIC, FONT_6X9, FONT_7X13, FONT_7X13_BOLD,
            FONT_7X13_ITALIC, FONT_7X14, FONT_7X14_BOLD, FONT_8X13, FONT_8X13_BOLD,
            FONT_8X13_ITALIC, FONT_9X15, FONT_9X15_BOLD, FONT_9X18, FONT_9X18_BOLD,
        },
        mapping::StrGlyphMapping,
        DecorationDimensions, MonoFont,
    },
    prelude::*,
};
use smart_leds::RGB8;

use crate::led_strip::{Frame1d as StripFrame, LedStrip, ToRgb888};
use crate::Result;

pub use embedded_graphics::geometry::{Point, Size};
pub use layout::LedLayout;

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

pub fn render_text_to_frame<const W: usize, const H: usize>(
    frame: &mut Frame2d<W, H>,
    font: &embedded_graphics::mono_font::MonoFont<'static>,
    text: &str,
    colors: &[RGB8],
    spacing_reduction: (i32, i32),
) -> Result<()> {
    let glyph_width = font.character_size.width as i32;
    let glyph_height = font.character_size.height as i32;
    let advance_x = glyph_width - spacing_reduction.0;
    let advance_y = glyph_height - spacing_reduction.1;
    let width_limit = W as i32;
    let height_limit = H as i32;
    if height_limit <= 0 || width_limit <= 0 {
        return Ok(());
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

        if x + advance_x > width_limit {
            continue;
        }

        let color = if colors.is_empty() {
            smart_leds::colors::WHITE
        } else {
            colors[color_index % colors.len()]
        };
        color_index = color_index.wrapping_add(1);

        let mut utf8 = [0u8; 4];
        let slice = ch.encode_utf8(&mut utf8);
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, color.to_rgb888());
        let position = embedded_graphics::prelude::Point::new(x, y);
        embedded_graphics::Drawable::draw(
            &embedded_graphics::text::Text::new(slice, position, style),
            frame,
        )
        .expect("drawing into frame cannot fail");

        x += advance_x;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub enum Led2dFont {
    Font3x4Trim,
    Font4x6,
    Font3x5Trim,
    Font5x7,
    Font4x6Trim,
    Font5x8,
    Font4x7Trim,
    Font6x9,
    Font5x8Trim,
    Font6x10,
    Font5x9Trim,
    Font6x12,
    Font5x11Trim,
    Font6x13,
    Font5x12Trim,
    Font6x13Bold,
    Font5x12TrimBold,
    Font6x13Italic,
    Font5x12TrimItalic,
    Font7x13,
    Font6x12Trim,
    Font7x13Bold,
    Font6x12TrimBold,
    Font7x13Italic,
    Font6x12TrimItalic,
    Font7x14,
    Font6x13Trim,
    Font7x14Bold,
    Font6x13TrimBold,
    Font8x13,
    Font7x12Trim,
    Font8x13Bold,
    Font7x12TrimBold,
    Font8x13Italic,
    Font7x12TrimItalic,
    Font9x15,
    Font8x14Trim,
    Font9x15Bold,
    Font8x14TrimBold,
    Font9x18,
    Font8x17Trim,
    Font9x18Bold,
    Font8x17TrimBold,
    Font10x20,
    Font9x19Trim,
}

impl Led2dFont {
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

#[derive(Clone, Copy, Debug)]
pub struct Frame2d<const W: usize, const H: usize>(pub [[RGB8; W]; H]);

impl<const W: usize, const H: usize> Frame2d<W, H> {
    pub const WIDTH: usize = W;
    pub const HEIGHT: usize = H;
    pub const LEN: usize = W * H;
    pub const SIZE: Size = Size::new(W as u32, H as u32);
    pub const TOP_LEFT: Point = Point::new(0, 0);
    pub const TOP_RIGHT: Point = Point::new((W - 1) as i32, 0);
    pub const BOTTOM_LEFT: Point = Point::new(0, (H - 1) as i32);
    pub const BOTTOM_RIGHT: Point = Point::new((W - 1) as i32, (H - 1) as i32);

    #[must_use]
    pub const fn new() -> Self {
        Self([[RGB8::new(0, 0, 0); W]; H])
    }

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

pub struct Led2d<'a, const N: usize, const MAX_FRAMES: usize> {
    led_strip: &'a LedStrip<N, MAX_FRAMES>,
    mapping_by_xy: [u16; N],
    width: usize,
}

impl<'a, const N: usize, const MAX_FRAMES: usize> Led2d<'a, N, MAX_FRAMES> {
    #[must_use]
    pub fn new<const W: usize, const H: usize>(
        led_strip: &'a LedStrip<N, MAX_FRAMES>,
        led_layout: &LedLayout<N, W, H>,
    ) -> Self {
        assert_eq!(
            W.checked_mul(H).expect("width * height must fit in usize"),
            N,
            "width * height must equal N"
        );
        Self {
            led_strip,
            mapping_by_xy: led_layout.xy_to_index(),
            width: W,
        }
    }

    #[must_use]
    fn xy_to_index(&self, x_index: usize, y_index: usize) -> usize {
        self.mapping_by_xy[y_index * self.width + x_index] as usize
    }

    fn convert_frame<const W: usize, const H: usize>(
        &self,
        frame_2d: Frame2d<W, H>,
    ) -> StripFrame<N> {
        let mut frame_1d = [RGB8::new(0, 0, 0); N];
        for y_index in 0..H {
            for x_index in 0..W {
                let led_index = self.xy_to_index(x_index, y_index);
                frame_1d[led_index] = frame_2d[(x_index, y_index)];
            }
        }
        StripFrame::from(frame_1d)
    }

    pub fn write_frame<const W: usize, const H: usize>(&self, frame: Frame2d<W, H>) {
        let strip_frame = self.convert_frame(frame);
        self.led_strip.write_frame(strip_frame);
    }

    pub fn animate<const W: usize, const H: usize, I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(Frame2d<W, H>, embassy_time::Duration)>,
    {
        self.led_strip.animate(frames.into_iter().map(|frame| {
            let (frame, duration) = *frame.borrow();
            (self.convert_frame(frame), duration)
        }));
    }
}

/// Generate a 2D panel device type backed by `led_strip!`.
///
/// Required fields:
/// - `len`
/// - `led_layout`
/// - `max_current`
/// - `font`
///
/// Optional fields:
/// - `engine` (default `Engine::Rmt`)
/// - `gamma`
/// - `max_frames`
#[macro_export]
macro_rules! led2d {
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: device_envoy_esp::led_strip::Engine::Spi
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: device_envoy_esp::led_strip::Engine::Rmt
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
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: $engine:path
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        compile_error!("led2d! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr
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
            [$led_layout],
            [$font],
        }
    };
}
