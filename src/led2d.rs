#![cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("led2d1", "docs/assets/led2d1.png"),
    doc = ::embed_doc_image::embed_image!("led2d2", "docs/assets/led2d2.png")
)]
//! A device abstraction for rectangular NeoPixel-style (WS2812) LED panel displays.
//!
//! See [`Led2dGenerated`](`crate::led2d::led2d_generated::Led2dGenerated`) for a
//! concrete generated-struct example and [`led2d!`] for the macro that builds these types.
//!
//! For custom graphics, create a [`Frame2d`] and use the
//! [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing API. See the
//! [`Frame2d`] documentation for an example.
//!
//! # Example: Write Text
//!
//! In this example, we render text on a 12×4 panel. Here, the generated struct is named `Led12x4`.
//!
//! ![LED panel preview][led2d1]
//!
//! ```no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::future;
//! # use core::result::Result::Ok;
//! # use embassy_executor::Spawner;
//! # use embassy_rp::init;
//! use device_kit::{Result, led2d, led2d::layout::LedLayout, led_strip::colors};
//!
//! // Tells us how the LED strip is wired up in the panel, in this case, a common snake-like
//! // pattern.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//!
//! // Generate a type named `Led12x4`.
//! led2d! {
//!     pub Led12x4,                         // Struct name and visibility
//!     pin: PIN_3,                          // GPIO pin for LED data signal
//!     width: 12,                           // Panel dimensions
//!     height: 4,
//!     led_layout: LED_LAYOUT_12X4,         // LED layout mapping
//!     font: Font3x4Trim,                   // Font variant
//! }
//!
//! # #[embassy_executor::main]
//! # pub async fn main(spawner: Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: Spawner) -> Result<Infallible> {
//!     let p = init(Default::default());
//!
//!     // Create a device abstraction for the LED panel.
//!     // Behind the scenes, this spawns a channel & background task to manage the display.
//!     let led12x4 = Led12x4::new(p.PIN_3, p.PIO0, p.DMA_CH0, spawner)?;
//!
//!     // Write text to the display with cycling colors.
//!     let colors = [colors::RED, colors::GREEN, colors::BLUE];
//!     led12x4.write_text("Rust", &colors).await?; // Colors cycle as needed.
//!
//!     future::pending().await // run forever
//! }
//! ```
//!
//! # Example: Animated Text on a Rotated Panel
//!
//! This example animates text on a rotated 12×8 panel built from two stacked 12×4 panels.
//!
//! ![LED panel preview][led2d2]
//!
//! ```no_run
//! # #![no_std]
//! # #![no_main]
//! # use panic_probe as _;
//! # use core::convert::Infallible;
//! # use core::future;
//! # use embassy_executor::Spawner;
//! # use embassy_rp::init;
//! use device_kit::{Result, led2d, led2d::layout::LedLayout, led2d::Frame2d, led_strip::{Current, Gamma, colors}};
//! use embassy_time::Duration;
//!
//! // Our panel is two 12x4 panels stacked vertically and then rotated clockwise.
//! const LED_LAYOUT_12X4: LedLayout<48, 12, 4> = LedLayout::serpentine_column_major();
//! const LED_LAYOUT_12X8: LedLayout<96, 12, 8> = LED_LAYOUT_12X4.concat_v(LED_LAYOUT_12X4);
//! const LED_LAYOUT_12X8_ROTATED: LedLayout<96, 8, 12> = LED_LAYOUT_12X8.rotate_cw();
//!
//! // Generate a type named `Led12x8Animated`.
//! led2d! {
//!     pub Led12x8Animated,
//!     pin: PIN_4,                           // GPIO pin for LED data signal
//!     width: 8,                             // Rotated panel width
//!     height: 12,                           // Rotated panel height
//!     led_layout: LED_LAYOUT_12X8_ROTATED,  // Two 12×4 panels stacked and rotated
//!     font: Font4x6Trim,                    // Use a 4x6 pixel font without the usual 1 pixel padding
//!     pio: PIO1,                            // PIO resource, default is PIO0
//!     dma: DMA_CH1,                         // DMA resource, default is DMA_CH0
//!     max_current: Current::Milliamps(300), // Power budget, default is 250 mA.
//!     gamma: Gamma::Linear,                 // Color correction curve, default is Gamma2_2
//!     max_frames: 2,                        // maximum animation frames, default is 16
//! }
//!
//! # #[embassy_executor::main]
//! # pub async fn main(spawner: Spawner) -> ! {
//! #     let err = example(spawner).await.unwrap_err();
//! #     core::panic!("{err}");
//! # }
//! async fn example(spawner: Spawner) -> Result<Infallible> {
//!     let p = init(Default::default());
//!
//!    // Create a device abstraction for the rotated LED panel.
//!     let led_12x8_animated = Led12x8Animated::new(p.PIN_4, p.PIO1, p.DMA_CH1, spawner)?;
//!
//!     // Write "Go" into an in-memory frame buffer.
//!     let mut frame_0 = Frame2d::new();
//!     // Empty text colors array defaults to white.
//!     led_12x8_animated.write_text_to_frame("Go", &[], &mut frame_0)?;
//!
//!     // Write "Go" into a second frame buffer with custom colors and on the 2nd line.
//!     let mut frame_1 = Frame2d::new();
//!     // "/n" starts a new line. Text does not wrap but rather clips.
//!     led_12x8_animated.write_text_to_frame(
//!         "\nGo",
//!         &[colors::HOT_PINK, colors::LIME],
//!         &mut frame_1,
//!     )?;
//!
//!     // Animate between the two frames indefinitely.
//!     let frame_duration = Duration::from_secs(1);
//!     led_12x8_animated
//!         .animate([(frame_0, frame_duration), (frame_1, frame_duration)])
//!         .await?;
//!
//!     future::pending().await // run forever
//! }
//! ```

// Re-export for macro use
#[doc(hidden)]
pub use paste;

// Re-export geometric types from embedded-graphics for convenience
pub use embedded_graphics::geometry::{Point, Size};

pub mod layout;

pub mod led2d_generated;

pub use layout::LedLayout;

#[cfg(not(feature = "host"))]
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
#[cfg(not(feature = "host"))]
use embassy_time::Timer;
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
    pixelcolor::Rgb888, // cmk should this just be color?
    prelude::*,
};
use heapless::Vec;
use smart_leds::RGB8;
use core::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

#[cfg(not(feature = "host"))]
use crate::led_strip::{Frame1d as StripFrame, LedStrip};
#[cfg(feature = "host")]
type StripFrame<const N: usize> = [RGB8; N];
use crate::Result;

/// Convert RGB8 (smart-leds) to Rgb888 (embedded-graphics).
#[must_use]
pub const fn rgb8_to_rgb888(color: RGB8) -> Rgb888 {
    Rgb888::new(color.r, color.g, color.b)
}

/// Convert Rgb888 (embedded-graphics) to RGB8 (smart-leds).
#[must_use]
pub fn rgb888_to_rgb8(color: Rgb888) -> RGB8 {
    RGB8::new(color.r(), color.g(), color.b())
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

#[doc(hidden)]
/// Monospace 3x4 font matching `bit_matrix3x4`.
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

#[doc(hidden)]
/// Render text into a frame using the provided font.
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

        // Clip characters that exceed width limit (no wrapping until explicit \n)
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
        let style = embedded_graphics::mono_font::MonoTextStyle::new(font, rgb8_to_rgb888(color));
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

/// Font options for [`Led2d`] text rendering.
///
/// Fonts with `Trim` suffix remove blank spacing to pack text more tightly on small displays.
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

    /// Return spacing reduction for trimmed variants (width, height).
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

// cmk0000000 needs links to led2d! and generated struct
/// A 2D array of RGB pixels representing a single display frame.
///
/// Frames are used to prepare images before sending them to the LED matrix. They support:
///
/// - Direct pixel access via array indexing
/// - Full graphics drawing via [`embedded-graphics`](https://docs.rs/embedded-graphics) (lines, shapes, text, and more)
/// - Automatic conversion to the strip's physical LED order
///
/// //cmk00000 if x and y are backwards, should/can we fix that?
/// Frames are stored in row-major order where `frame[row][col]` represents the pixel
/// at display coordinates (col, row). The physical mapping to the LED strip is handled
/// automatically by the device abstraction.
///
/// cmk000000000 do the generated structs also have these associated constants and documentation?
/// cmk000000000 does the 1d rame and generated structs need them too?
/// # Associated Constants
///
/// - `WIDTH` — Frame2d width in pixels (columns)
/// - `HEIGHT` — Frame2d height in pixels (rows)
/// - `LEN` — Total pixel count (WIDTH × HEIGHT)
///
///  *For [`embedded-graphics`](https://docs.rs/embedded-graphics) drawing operations:*
/// - `SIZE` — [`Size`] frame dimensions
/// - `TOP_LEFT`, `TOP_RIGHT`, `BOTTOM_LEFT`, `BOTTOM_RIGHT` — Corner [`Point`] coordinates
///
/// # Example: Draw pixels both directly and with [`embedded-graphics`](https://docs.rs/embedded-graphics):
///
/// ![LED panel preview][led2d-graphics]
///
/// // cmk00000 we need to tell about fonts, and the coordinate system, and say us EB for fancy font stuff
///
/// ```no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_kit::led2d::Frame2d;
/// use embedded_graphics::{
///     pixelcolor::Rgb888,
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
/// Rectangle::new(Frame::TOP_LEFT, Frame::SIZE)
///     .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
///     .draw(&mut frame)
///     .expect("rectangle draw must succeed");
///
/// // Direct pixel access: set the upper-left LED pixel (x = 0, y = 0).
/// // Frame2d stores LED colors directly, so we write an LED color here.
/// frame[0][0] = colors::CYAN;
///
/// // Use the embedded-graphics crate to draw a green circle centered in the frame.
/// const DIAMETER: u32 = 6;
/// const CIRCLE_TOP_LEFT: Point = centered_top_left(Frame::WIDTH, Frame::HEIGHT, DIAMETER as usize);
/// Circle::new(CIRCLE_TOP_LEFT, DIAMETER)
///     .into_styled(PrimitiveStyle::with_stroke(Rgb888::GREEN, 1))
///     .draw(&mut frame)
///     .expect("circle draw must succeed");
/// # }
/// ```

#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("led2d-graphics", "docs/assets/led2d_graphics.png")
)]
#[derive(Clone, Copy, Debug)]
pub struct Frame2d<const W: usize, const H: usize>(pub [[RGB8; W]; H]);

impl<const W: usize, const H: usize> Frame2d<W, H> {
    // cmk00000 are all these constants still needed?
    /// Frame2d width in pixels (columns).
    pub const WIDTH: usize = W;
    /// Frame2d height in pixels (rows).
    pub const HEIGHT: usize = H;
    /// Total number of pixels (WIDTH × HEIGHT).
    pub const LEN: usize = W * H;
    /// Frame2d dimensions as a [`Size`] for embedded-graphics.
    pub const SIZE: Size = Size::new(W as u32, H as u32);
    /// Top-left corner coordinate.
    pub const TOP_LEFT: Point = Point::new(0, 0);
    /// Top-right corner coordinate.
    pub const TOP_RIGHT: Point = Point::new((W - 1) as i32, 0);
    /// Bottom-left corner coordinate.
    pub const BOTTOM_LEFT: Point = Point::new(0, (H - 1) as i32);
    /// Bottom-right corner coordinate.
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
            let column_index = coord.x;
            let row_index = coord.y;
            if column_index >= 0
                && column_index < W as i32
                && row_index >= 0
                && row_index < H as i32
            {
                self.0[row_index as usize][column_index as usize] =
                    RGB8::new(color.r(), color.g(), color.b());
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
// Public so macro expansions in downstream crates can share the command signal type.
pub type Led2dCommandSignal<const N: usize, const MAX_FRAMES: usize> =
    Signal<CriticalSectionRawMutex, Command<N, MAX_FRAMES>>;
#[doc(hidden)]
// Public so macro expansions in downstream crates can observe completion signals.
pub type Led2dCompletionSignal = Signal<CriticalSectionRawMutex, ()>;

#[doc(hidden)]
// Public so macro-generated tasks can share the command channel type.
/// Command for the LED device loop.
#[derive(Clone)]
pub enum Command<const N: usize, const MAX_FRAMES: usize> {
    DisplayStatic(StripFrame<N>),
    Animate(Vec<(StripFrame<N>, Duration), MAX_FRAMES>),
}

/// Static type for the [`Led2d`] device abstraction.
///
/// Most users should use the `led2d!` or `led2d_from_strip!` macros which generate
/// a higher-level wrapper.
pub struct Led2dStatic<const N: usize, const MAX_FRAMES: usize> {
    pub command_signal: Led2dCommandSignal<N, MAX_FRAMES>,
    pub completion_signal: Led2dCompletionSignal,
}

impl<const N: usize, const MAX_FRAMES: usize> Led2dStatic<N, MAX_FRAMES> {
    #[must_use]
    pub const fn new_static() -> Self {
        Self {
            command_signal: Signal::new(),
            completion_signal: Signal::new(),
        }
    }
}

// cmk000 don't use the phrase 'module-level' in docs.
// cmk00 this needs a compiled-only doc test.
/// A device abstraction for rectangular NeoPixel-style (WS2812) LED matrix displays.
///
/// Supports any size display with arbitrary LED-index-to-coordinate mapping. The provided mapping
/// is reversed during initialization into an internal (row, col) → LED index lookup so frame
/// conversion stays fast.
///
/// Rows and columns are metadata used only for indexing - the core type is generic only over
/// N (total LEDs) and MAX_FRAMES (animation capacity).
///
/// Most users should use the `led2d!` or `led2d_from_strip!` macros which generate
/// a higher-level wrapper. See the [mod@crate::led2d] module docs for examples.
pub struct Led2d<const N: usize, const MAX_FRAMES: usize> {
    command_signal: &'static Led2dCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static Led2dCompletionSignal,
    mapping_by_xy: [u16; N],
    width: usize,
}

impl<const N: usize, const MAX_FRAMES: usize> Led2d<N, MAX_FRAMES> {
    /// Create Led2d device handle.
    ///
    /// The `led_layout` defines how LED indices map to `(column, row)` coordinates. Entry `i`
    /// provides the `(col, row)` destination for LED `i`. The layout is inverted via
    /// [`LedLayout::mapping_by_xy`] so (row, col) queries are O(1) when converting frames.
    ///
    /// See the [struct-level example](Self) for usage.
    #[must_use]
    pub fn new<const W: usize, const H: usize>(
        led2d_static: &'static Led2dStatic<N, MAX_FRAMES>,
        led_layout: &LedLayout<N, W, H>,
    ) -> Self {
        assert_eq!(
            W.checked_mul(H).expect("width * height must fit in usize"),
            N,
            "width * height must equal N (total LEDs for led_layout reversal)"
        );
        Self {
            command_signal: &led2d_static.command_signal,
            completion_signal: &led2d_static.completion_signal,
            mapping_by_xy: led_layout.mapping_by_xy(),
            width: W,
        }
    }

    /// Convert (column, row) coordinates to LED strip index using the stored LED layout.
    #[must_use]
    fn xy_to_index(&self, column_index: usize, row_index: usize) -> usize {
        self.mapping_by_xy[row_index * self.width + column_index] as usize
    }

    /// Convert 2D frame to 1D array using the LED layout.
    fn convert_frame<const W: usize, const H: usize>(
        &self,
        frame_2d: Frame2d<W, H>,
    ) -> StripFrame<N> {
        let mut frame_1d = [RGB8::new(0, 0, 0); N];
        for row_index in 0..H {
            for column_index in 0..W {
                let led_index = self.xy_to_index(column_index, row_index);
                frame_1d[led_index] = frame_2d[row_index][column_index];
            }
        }
        StripFrame::from(frame_1d)
    }

    /// Render a fully defined frame to the panel.
    ///
    /// Frame2d is a 2D array in row-major order where `frame[row][col]` is the pixel at (col, row).
    pub async fn write_frame<const W: usize, const H: usize>(
        &self,
        frame: Frame2d<W, H>,
    ) -> Result<()> {
        let strip_frame = self.convert_frame(frame);
        self.command_signal
            .signal(Command::DisplayStatic(strip_frame));
        self.completion_signal.wait().await;
        Ok(())
    }

    /// Loop through a sequence of animation frames until interrupted by another command.
    ///
    /// Each frame is a tuple of `(Frame2d, Duration)`. Accepts arrays, `Vec`s, or any
    /// iterator that produces `(Frame2d, Duration)` tuples. For best efficiency with large
    /// frame sequences, pass an iterator to avoid intermediate allocations.
    pub async fn animate<const W: usize, const H: usize>(
        &self,
        frames: impl IntoIterator<Item = (Frame2d<W, H>, Duration)>,
    ) -> Result<()> {
        assert!(
            MAX_FRAMES > 0,
            "max_frames must be positive for Led2d animations"
        );
        let mut sequence: Vec<(StripFrame<N>, Duration), MAX_FRAMES> = Vec::new();
        for (frame, duration) in frames {
            assert!(
                duration.as_micros() > 0,
                "animation frame duration must be positive"
            );
            let strip_frame = self.convert_frame(frame);
            sequence
                .push((strip_frame, duration))
                .expect("animation sequence fits");
        }
        assert!(
            !sequence.is_empty(),
            "animation requires at least one frame"
        );
        defmt::info!("Led2d::animate: sending {} frames", sequence.len());
        self.command_signal.signal(Command::Animate(sequence));
        defmt::info!("Led2d::animate: waiting for completion");
        self.completion_signal.wait().await;
        defmt::info!("Led2d::animate: completed (animation started)");
        Ok(())
    }
}

// Must be `pub` (not `pub(crate)`) because called by macro-generated code that expands at the call site in downstream crates.
// This is an implementation detail, not part of the user-facing API.
#[doc(hidden)]
/// Device loop for Led2d. Called by macro-generated code.
///
/// Since embassy tasks cannot be generic, the macros generate a concrete wrapper task
/// that calls this function. Must be `pub` because macro expansion happens in the calling
/// crate's context, but hidden from docs as it's not part of the public API.
#[cfg(not(feature = "host"))]
pub async fn led2d_device_loop<const N: usize, const MAX_FRAMES: usize, S>(
    command_signal: &'static Led2dCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static Led2dCompletionSignal,
    led_strip: S,
) -> Result<Infallible>
where
    S: AsRef<LedStrip<N, MAX_FRAMES>>,
{
    defmt::info!("led2d_device_loop: task started");
    let led_strip_ref = led_strip.as_ref();
    loop {
        defmt::debug!("led2d_device_loop: waiting for command");
        let command = command_signal.wait().await;
        command_signal.reset();

        match command {
            Command::DisplayStatic(frame) => {
                led_strip_ref.write_frame(frame).await?;
                completion_signal.signal(());
            }
            Command::Animate(frames) => {
                defmt::info!(
                    "led2d_device_loop: received Animate command with {} frames",
                    frames.len()
                );
                let next_command =
                    run_animation_loop(frames, command_signal, completion_signal, led_strip_ref)
                        .await?;
                defmt::info!("led2d_device_loop: animation interrupted");
                match next_command {
                    Command::DisplayStatic(frame) => {
                        defmt::info!(
                            "led2d_device_loop: processing DisplayStatic from animation interrupt"
                        );
                        led_strip_ref.write_frame(frame).await?;
                        completion_signal.signal(());
                    }
                    Command::Animate(new_frames) => {
                        defmt::info!("led2d_device_loop: restarting with new animation");
                        // Process the new animation immediately without waiting for next command
                        let next_command = run_animation_loop(
                            new_frames,
                            command_signal,
                            completion_signal,
                            led_strip_ref,
                        )
                        .await?;
                        // Handle any command that interrupted this animation
                        match next_command {
                            Command::DisplayStatic(frame) => {
                                led_strip_ref.write_frame(frame).await?;
                                completion_signal.signal(());
                            }
                            Command::Animate(_) => {
                                // Another animation interrupted; loop back to handle it
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "host"))]
async fn run_animation_loop<const N: usize, const MAX_FRAMES: usize>(
    frames: Vec<(StripFrame<N>, Duration), MAX_FRAMES>,
    command_signal: &'static Led2dCommandSignal<N, MAX_FRAMES>,
    completion_signal: &'static Led2dCompletionSignal,
    led_strip: &LedStrip<N, MAX_FRAMES>,
) -> Result<Command<N, MAX_FRAMES>>
{
    defmt::info!("run_animation_loop: starting with {} frames", frames.len());
    completion_signal.signal(());
    defmt::debug!("run_animation_loop: signaled completion (animation started)");

    loop {
        for (frame_index, (strip_frame, duration)) in frames.iter().enumerate() {
            defmt::trace!("run_animation_loop: displaying frame {}", frame_index);
            led_strip.write_frame(*strip_frame).await?;

            match select(command_signal.wait(), Timer::after(*duration)).await {
                Either::First(new_command) => {
                    defmt::info!("run_animation_loop: received new command, interrupting");
                    command_signal.reset();
                    return Ok(new_command);
                }
                Either::Second(()) => continue,
            }
        }
        defmt::debug!("run_animation_loop: completed one loop, restarting");
    }
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! led2d_device_task {
    (
        $task_name:ident,
        $strip_ty:ty,
        $n:expr,
        $max_frames:expr $(,)?
    ) => {
        $crate::led2d::led2d_device_task!(
            @inner
            ()
            $task_name,
            $strip_ty,
            $n,
            $max_frames
        );
    };
    (
        $vis:vis $task_name:ident,
        $strip_ty:ty,
        $n:expr,
        $max_frames:expr $(,)?
    ) => {
        $crate::led2d::led2d_device_task!(
            @inner
            ($vis)
            $task_name,
            $strip_ty,
            $n,
            $max_frames
        );
    };
    (
        @inner
        ($($vis:tt)*)
        $task_name:ident,
        $strip_ty:ty,
        $n:expr,
        $max_frames:expr $(,)?
    ) => {
        #[embassy_executor::task]
        #[allow(non_snake_case)]
        $($vis)* async fn $task_name(
            command_signal: &'static $crate::led2d::Led2dCommandSignal<$n, $max_frames>,
            completion_signal: &'static $crate::led2d::Led2dCompletionSignal,
            led_strip: $strip_ty,
        ) {
            let err =
                $crate::led2d::led2d_device_loop(command_signal, completion_signal, led_strip)
                    .await
                    .unwrap_err();
            panic!("{err}");
        }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "host"))]
pub use led2d_device_task;

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! led2d_device {
    (
        $vis:vis struct $resources_name:ident,
        task: $task_vis:vis $task_name:ident,
        strip: $strip_ty:ty,
        leds: $n:expr,
        led_layout: $led_layout:expr,
        width: $width:expr,
        max_frames: $max_frames:expr $(,)?
    ) => {
        $crate::led2d::led2d_device_task!($task_vis $task_name, $strip_ty, $n, $max_frames);

        $vis struct $resources_name {
            led2d_static: $crate::led2d::Led2dStatic<$n, $max_frames>,
        }

        impl $resources_name {
            /// Create the static resources for this Led2d instance.
            #[must_use]
            pub const fn new_static() -> Self {
                Self {
                    led2d_static: $crate::led2d::Led2dStatic::new_static(),
                }
            }

            /// Construct the `Led2d` handle, spawning the background task automatically.
            pub fn new(
                &'static self,
                led_strip: $strip_ty,
                spawner: ::embassy_executor::Spawner,
            ) -> $crate::Result<$crate::led2d::Led2d<$n, $max_frames>> {
                let token = $task_name(
                    &self.led2d_static.command_signal,
                    &self.led2d_static.completion_signal,
                    led_strip,
                );
                spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                let height = $n / $width;
                assert_eq!(
                    height * $width,
                    $n,
                    "width must evenly divide total LED count to derive height"
                );
                Ok($crate::led2d::Led2d::new(
                    &self.led2d_static,
                    &$led_layout,
                ))
            }
        }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "host"))]
pub use led2d_device;

/// Macro to generate a device abstraction for a NeoPixel-style (WS2812) 2D LED panel.
///
/// See the [module documentation](mod@crate::led2d) for usage examples.
///
/// **Required fields:**
///
/// - `pin` — GPIO pin for LED data
/// - `width` — Number of columns in the panel
/// - `height` — Number of rows in the panel
/// - `led_layout` — LED strip physical layout (see [`LedLayout`])
/// - `font` — Built-in font variant (see [`Led2dFont`])
///
/// **Optional fields:**
///
/// - `pio` — PIO resource to use (default: `PIO0`)
/// - `dma` — DMA channel (default: `DMA_CH0`)
/// - `max_current` — Current budget (default: [`crate::led_strip::MAX_CURRENT_DEFAULT`] = 250 mA)
/// - `gamma` — Color curve (default: [`crate::led_strip::GAMMA_DEFAULT`] = `Gamma::Gamma2_2`)
/// - `max_frames` — Maximum animation frames for the generated strip (default: [`crate::led_strip::MAX_FRAMES_DEFAULT`] = 16)
///
/// # Current Limiting
///
/// The `max_current` field automatically scales brightness to stay within your power budget.
///
/// Each WS2812 LED is assumed to draw 60 mA at full brightness. For example:
/// - 16 LEDs × 60 mA = 960 mA at full brightness
/// - With `max_current: Current::Milliamps(1000)`, all LEDs fit at 100% brightness
/// - With `max_current: crate::led_strip::MAX_CURRENT_DEFAULT` (250 mA by default), the generated `MAX_BRIGHTNESS` limits LEDs to ~26% brightness
///
/// The current limit is baked into a compile-time lookup table, so it has no
/// runtime cost.
///
/// **Powering LEDs from the Pico's pin 40 (VBUS):** Pin 40 is the USB 5 V rail
/// pass-through, but the Pico itself has practical current limits — the USB connector,
/// cable, and internal circuitry aren't designed for heavy loads. Small LED panels
/// (a few hundred mA) can usually power from pin 40 with a decent USB supply; for
/// larger loads (1 A+), use a separate 5 V supply and share ground with the Pico.
///
/// # Color Correction (Gamma)
///
/// The `gamma` field applies a color response curve to make colors look more natural:
///
/// - [`Gamma::Linear`](`crate::led_strip::Gamma::Linear`) — No correction (raw values)
/// - [`Gamma::Gamma2_2`](`crate::led_strip::Gamma::Gamma2_2`) — Standard sRGB curve (default, most natural-looking; see [`crate::led_strip::GAMMA_DEFAULT`])
///
/// The gamma curve is baked into a compile-time lookup table, so it has no
/// runtime cost.
///
/// # When to Use This Macro
///
/// Use `led2d!` when you want a **single LED panel** generated from a strip.
///
/// If you need to share a PIO resource or build from an existing strip, use
/// [`led_strips!`](crate::led_strip::led_strips)'s led2d feature instead.
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! led2d {
    ($($tt:tt)*) => { $crate::__led2d_impl! { $($tt)* } };
}

/// Implementation macro. Not part of the public API; use [`led2d!`] instead.
#[doc(hidden)] // Required pub for macro expansion in downstream crates
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! __led2d_impl {
    (
        $vis:vis $name:ident,
        $($fields:tt)*
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: PIO0,
            pin: _UNSET_,
            dma: DMA_CH0,
            width: _UNSET_,
            height: _UNSET_,
            led_layout: _UNSET_,
            max_current: _UNSET_,
            gamma: $crate::led_strip::GAMMA_DEFAULT,
            max_frames: $crate::led_strip::MAX_FRAMES_DEFAULT,
            font: _UNSET_,
            fields: [ $($fields)* ]
        }
    };

    // Fill defaults: pio
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ pio: $new_pio:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $new_pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: pin
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ pin: $new_pin:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $new_pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: dma
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ dma: $new_dma:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $new_dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: width
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ width: $new_width:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $new_width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: height
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ height: $new_height:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $new_height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: led_layout
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ led_layout: $new_led_layout:tt $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $new_led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_current
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ max_current: $new_max_current:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $new_max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: gamma
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ gamma: $new_gamma:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $new_gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: max_frames
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ max_frames: $new_max_frames:expr $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $new_max_frames,
            font: $font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill defaults: font
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:tt,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ font: $new_font_variant:ident $(, $($rest:tt)* )? ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $new_font_variant,
            fields: [ $($($rest)*)? ]
        }
    };

    // Fill default max_current if still unset.
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: _UNSET_,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:tt,
        fields: [ ]
    ) => {
        $crate::__led2d_impl! {
            @__fill_defaults
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $crate::led_strip::MAX_CURRENT_DEFAULT,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant,
            fields: [ ]
        }
    };

    // Terminal: pass through once all fields consumed.
    (@__fill_defaults
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:tt,
        dma: $dma:ident,
        width: $width:tt,
        height: $height:tt,
        led_layout: $led_layout:tt,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:ident,
        fields: [ ]
    ) => {
        $crate::__led2d_impl! {
            @__expand
            vis: $vis,
            name: $name,
            pio: $pio,
            pin: $pin,
            dma: $dma,
            width: $width,
            height: $height,
            led_layout: $led_layout,
            max_current: $max_current,
            gamma: $gamma,
            max_frames: $max_frames,
            font: $font_variant
        }
    };

    // Expand: serpentine column-major led_layout variant.
    (@__expand
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: serpentine_column_major,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:ident
    ) => {
        $crate::led2d::paste::paste! {
            // Generate the LED strip infrastructure with a CamelCase strip type
            $crate::__led_strips_impl! {
                @__with_frame_alias
                frame_alias: __SKIP_FRAME_ALIAS__,
                pio: $pio,
                [<$name Strips>] {
                    [<$name:snake>]: {
                        dma: $dma,
                        pin: $pin,
                        len: { $width * $height },
                        max_current: $max_current,
                        gamma: $gamma,
                        max_frames: $max_frames,
                    }
                }
            }

            // Generate the Led2d device from the strip
            $crate::led2d::led2d_from_strip! {
                $vis $name,
                strip_type: [<$name LedStrip>],
                width: $width,
                height: $height,
                led_layout: serpentine_column_major,
                font: $font_variant,
            }

            // Add simplified constructor that handles PIO splitting and both statics
            impl [<$name>] {
                /// Create a new LED matrix display with automatic PIO setup.
                ///
                /// This is a convenience constructor that handles PIO splitting and static
                /// resource management automatically. All initialization happens in a single call.
                ///
                /// # Parameters
                ///
                /// - `pin`: GPIO pin for LED data signal
                /// - `pio`: PIO peripheral
                /// - `dma`: DMA channel for LED data transfer
                /// - `spawner`: Task spawner for background operations
                #[allow(non_upper_case_globals)]
                $vis fn new(
                    pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                    pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                    dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<Self> {
                    // Split PIO into state machines (uses SM0 automatically)
                    let (sm0, _sm1, _sm2, _sm3) = [<$pio:lower _split>](pio);

                    // Create strip (uses interior static)
                    let strip = [<$name LedStrip>]::new(
                        sm0,
                        pin,
                        dma,
                        spawner
                    )?;

                    // Create Led2d from strip (uses interior static)
                    [<$name>]::from_strip(strip, spawner)
                }
            }
        }
    };

    // Expand: custom led_layout variant (LedLayout expression).
    (@__expand
        vis: $vis:vis,
        name: $name:ident,
        pio: $pio:ident,
        pin: $pin:ident,
        dma: $dma:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: $led_layout:expr,
        max_current: $max_current:expr,
        gamma: $gamma:expr,
        max_frames: $max_frames:expr,
        font: $font_variant:ident
    ) => {
        $crate::led2d::paste::paste! {
            // Generate the LED strip infrastructure with a CamelCase strip type
            $crate::__led_strips_impl! {
                @__with_frame_alias
                frame_alias: __SKIP_FRAME_ALIAS__,
                pio: $pio,
                [<$name Strips>] {
                    [<$name:snake>]: {
                        dma: $dma,
                        pin: $pin,
                        len: $width * $height,
                        max_current: $max_current,
                        gamma: $gamma,
                        max_frames: $max_frames,
                    }
                }
            }

            // Generate the Led2d device from the strip with custom mapping
            $crate::led2d::led2d_from_strip! {
                $vis $name,
                strip_type: [<$name LedStrip>],
                width: $width,
                height: $height,
                led_layout: $led_layout,
                font: $font_variant,
            }

            // Add simplified constructor that handles PIO splitting and both statics
            #[allow(non_snake_case, dead_code)]
            impl [<$name>] {
                /// Create a new LED matrix display with automatic PIO setup.
                ///
                /// This is a convenience constructor that handles PIO splitting and static
                /// resource management automatically. All initialization happens in a single call.
                ///
                /// # Parameters
                ///
                /// - `pin`: GPIO pin for LED data signal
                /// - `pio`: PIO peripheral
                /// - `dma`: DMA channel for LED data transfer
                /// - `spawner`: Task spawner for background operations
                #[allow(non_upper_case_globals)]
                $vis fn new(
                    pin: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pin>,
                    pio: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$pio>,
                    dma: ::embassy_rp::Peri<'static, ::embassy_rp::peripherals::$dma>,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<Self> {
                    // Split PIO into state machines (uses SM0 automatically)
                    let (sm0, _sm1, _sm2, _sm3) = [<$pio:lower _split>](pio);

                    // Create strip (uses interior static)
                    let led_strip = [<$name LedStrip>]::new(
                        sm0,
                        pin,
                        dma,
                        spawner
                    )?;

                    // Create Led2d from strip (uses interior static)
                    [<$name>]::from_strip(led_strip, spawner)
                }
            }
        }
    };
}

/// Generate a Led2d device abstraction from an existing LED strip type.
///
/// Use this macro when you want to share a PIO across multiple LED strips and treat one as a 2D display.
/// For simple single-strip displays, use `led2d!` instead.
/// The strip must be created with [`led_strips!`](crate::led_strip::led_strips).
///
/// # Parameters
///
/// - Visibility and base name for generated types (e.g., `pub Led12x4`)
/// - `strip_type` - Name of the strip type created by `led_strips!`
/// - `width` - Number of columns in the panel
/// - `height` - Number of rows in the panel
/// - `led_layout` - LED strip physical layout:
///   - `serpentine_column_major` - Common serpentine wiring pattern (LED index → `(col, row)`)
///   - `LedLayout` expression - Custom LED layout value in LED-index order
/// - `font` - Built-in font variant (see [`Led2dFont`])
///
/// # Example
///
/// ```no_run
/// # #![no_std]
/// # #![no_main]
/// # use panic_probe as _;
/// use device_kit::led_strip::led_strips;
/// use device_kit::led2d::led2d_from_strip;
/// use device_kit::led_strip::Current;
/// use device_kit::led_strip::Gamma;
/// use device_kit::pio_split;
/// use embassy_executor::Spawner;
/// use embassy_time::Duration;
///
/// // Define multiple strips sharing PIO1
/// led_strips! {
///     pio: PIO1,
///     Led12x4Strips {
///         strip: {
///             pin: PIN_3,
///             len: 48,
///             max_current: Current::Milliamps(500),
///             gamma: Gamma::Linear,
///         }
///     }
/// }
///
/// // Wrap the strip as a Led2d surface
/// led2d_from_strip! {
///     pub Led12x4,
///     strip_type: StripLedStrip,
///     width: 12,
///     height: 4,
///     led_layout: serpentine_column_major,
///     font: Font3x4Trim,
/// }
///
/// # #[embassy_executor::main]
/// # async fn main(spawner: Spawner) {
/// #     let p = embassy_rp::init(Default::default());
/// #     let (sm0, _sm1, _sm2, _sm3) = pio_split!(p.PIO1);
/// #     let strip = StripLedStrip::new(sm0, p.PIN_3, p.DMA_CH0, spawner).unwrap();
/// #     let led = Led12x4::from_strip(strip, spawner).unwrap();
/// # }
/// ```
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
#[macro_export]
#[cfg(not(feature = "host"))]
macro_rules! led2d_from_strip {
    // Serpentine column-major led_layout variant (uses strip's MAX_FRAMES)
    (
        $vis:vis $name:ident,
        strip_type: $strip_type:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: serpentine_column_major,
        font: $font_variant:ident $(,)?
    ) => {
        $crate::led2d::paste::paste! {
            const [<$name:upper _W>]: usize = $width;
            const [<$name:upper _H>]: usize = $height;
            const [<$name:upper _N>]: usize = [<$name:upper _W>] * [<$name:upper _H>];
            const [<$name:upper _LED_LAYOUT>]: $crate::led2d::LedLayout<[<$name:upper _N>], [<$name:upper _W>], [<$name:upper _H>]> =
                $crate::led2d::LedLayout::<[<$name:upper _N>], [<$name:upper _W>], [<$name:upper _H>]>::serpentine_column_major();
            const [<$name:upper _MAX_FRAMES>]: usize = $strip_type::MAX_FRAMES;

            // Compile-time assertion that strip length matches led_layout length
            const _: () = assert!([<$name:upper _LED_LAYOUT>].map().len() == $strip_type::LEN);

            $crate::led2d::led2d_from_strip!(
                @common $vis, $name, $strip_type, [<$name:upper _W>], [<$name:upper _H>], [<$name:upper _N>], [<$name:upper _LED_LAYOUT>],
                $font_variant,
                [<$name:upper _MAX_FRAMES>]
            );
        }
    };
    // Custom led_layout variant (uses strip's MAX_FRAMES)
    (
        $vis:vis $name:ident,
        strip_type: $strip_type:ident,
        width: $width:expr,
        height: $height:expr,
        led_layout: $led_layout:expr,
        font: $font_variant:ident $(,)?
    ) => {
        $crate::led2d::paste::paste! {
            const [<$name:upper _W>]: usize = $width;
            const [<$name:upper _H>]: usize = $height;
            const [<$name:upper _N>]: usize = [<$name:upper _W>] * [<$name:upper _H>];
            const [<$name:upper _LED_LAYOUT>]: $crate::led2d::LedLayout<[<$name:upper _N>], [<$name:upper _W>], [<$name:upper _H>]> = $led_layout;
            const [<$name:upper _MAX_FRAMES>]: usize = $strip_type::MAX_FRAMES;

            // Compile-time assertion that strip length matches led_layout length
            const _: () = assert!([<$name:upper _LED_LAYOUT>].map().len() == $strip_type::LEN);

            $crate::led2d::led2d_from_strip!(
                @common $vis, $name, $strip_type, [<$name:upper _W>], [<$name:upper _H>], [<$name:upper _N>], [<$name:upper _LED_LAYOUT>],
                $font_variant,
                [<$name:upper _MAX_FRAMES>]
            );
        }
    };
    // Common implementation (shared by both variants)
    (
        @common $vis:vis,
        $name:ident,
        $strip_type:ident,
        $cols_const:ident,
        $rows_const:ident,
        $n_const:ident,
        $led_layout_const:ident,
        $font_variant:expr,
        $max_frames_const:ident
    ) => {
        $crate::led2d::paste::paste! {
            /// Static resources for the LED matrix device.
            $vis struct [<$name Static>] {
                led2d_static: $crate::led2d::Led2dStatic<$n_const, $max_frames_const>,
            }

            // Generate the task wrapper
            $crate::led2d::led2d_device_task!(
                [<$name:snake _device_loop>],
                &'static $strip_type,
                $n_const,
                $max_frames_const
            );

            /// LED matrix device handle generated by [`led2d_from_strip!`](crate::led2d::led2d_from_strip).
            $vis struct [<$name>] {
                led2d: $crate::led2d::Led2d<$n_const, $max_frames_const>,
                font: embedded_graphics::mono_font::MonoFont<'static>,
                font_variant: $crate::led2d::Led2dFont,
            }

            #[allow(non_snake_case, dead_code)]
            impl [<$name>] {
                /// Number of columns in the panel.
                pub const WIDTH: usize = $cols_const;
                /// Number of rows in the panel.
                pub const HEIGHT: usize = $rows_const;
                /// Total number of LEDs (WIDTH * HEIGHT).
                pub const N: usize = $n_const;
                /// Frame dimensions as a [`Size`] for embedded-graphics.
                pub const SIZE: $crate::led2d::Size = $crate::led2d::Frame2d::<$cols_const, $rows_const>::SIZE;
                /// Top-left corner coordinate for embedded-graphics drawing.
                pub const TOP_LEFT: $crate::led2d::Point = $crate::led2d::Frame2d::<$cols_const, $rows_const>::TOP_LEFT;
                /// Top-right corner coordinate for embedded-graphics drawing.
                pub const TOP_RIGHT: $crate::led2d::Point = $crate::led2d::Frame2d::<$cols_const, $rows_const>::TOP_RIGHT;
                /// Bottom-left corner coordinate for embedded-graphics drawing.
                pub const BOTTOM_LEFT: $crate::led2d::Point = $crate::led2d::Frame2d::<$cols_const, $rows_const>::BOTTOM_LEFT;
                /// Bottom-right corner coordinate for embedded-graphics drawing.
                pub const BOTTOM_RIGHT: $crate::led2d::Point = $crate::led2d::Frame2d::<$cols_const, $rows_const>::BOTTOM_RIGHT;
                /// Maximum animation frames supported for this device.
                pub const MAX_FRAMES: usize = $max_frames_const;

                /// Create static resources.
                #[must_use]
                $vis const fn new_static() -> [<$name Static>] {
                    [<$name Static>] {
                        led2d_static: $crate::led2d::Led2dStatic::new_static(),
                    }
                }

                /// Create a new LED matrix display instance from an existing strip.
                ///
                /// The strip must be created from the same type specified in `strip_type`.
                /// For simpler single-strip setups, see the convenience constructor generated
                /// by [`led2d!`].
                ///
                /// # Parameters
                ///
                /// - `led_strip`: LED strip instance from the specified strip type
                /// - `spawner`: Task spawner for background operations
                $vis fn from_strip(
                    led_strip: &'static $strip_type,
                    spawner: ::embassy_executor::Spawner,
                ) -> $crate::Result<Self> {
                    static STATIC: [<$name Static>] = [<$name>]::new_static();

                    defmt::info!("Led2d::new: spawning device task");
                    let token = [<$name:snake _device_loop>](
                        &STATIC.led2d_static.command_signal,
                        &STATIC.led2d_static.completion_signal,
                        led_strip,
                    );
                    spawner.spawn(token).map_err($crate::Error::TaskSpawn)?;
                    defmt::info!("Led2d::new: device task spawned");

                    let led2d = $crate::led2d::Led2d::new(
                        &STATIC.led2d_static,
                        &$led_layout_const,
                    );

                    defmt::info!("Led2d::new: device created successfully");
                    Ok(Self {
                        led2d,
                        font: $crate::led2d::Led2dFont::$font_variant.to_font(),
                        font_variant: $crate::led2d::Led2dFont::$font_variant,
                    })
                }

                /// Render a fully defined frame to the panel.
                $vis async fn write_frame(&self, frame: $crate::led2d::Frame2d<$cols_const, $rows_const>) -> $crate::Result<()> {
                    self.led2d.write_frame(frame).await
                }

                /// Loop through a sequence of animation frames. Pass arrays by value or Vecs/iters.
                $vis async fn animate(&self, frames: impl IntoIterator<Item = ($crate::led2d::Frame2d<$cols_const, $rows_const>, ::embassy_time::Duration)>) -> $crate::Result<()> {
                    self.led2d.animate(frames).await
                }

                /// Render text into a frame using the configured font and spacing.
                pub fn write_text_to_frame(
                    &self,
                    text: &str,
                    colors: &[smart_leds::RGB8],
                    frame: &mut $crate::led2d::Frame2d<$cols_const, $rows_const>,
                ) -> $crate::Result<()> {
                    $crate::led2d::render_text_to_frame(frame, &self.font, text, colors, self.font_variant.spacing_reduction())
                }

                /// Render text and display it on the LED matrix.
                pub async fn write_text(&self, text: &str, colors: &[smart_leds::RGB8]) -> $crate::Result<()> {
                    let mut frame = $crate::led2d::Frame2d::<$cols_const, $rows_const>::new();
                    self.write_text_to_frame(text, colors, &mut frame)?;
                    self.write_frame(frame).await
                }
            }
        }
    };
}

#[cfg(not(feature = "host"))]
#[doc(inline)]
pub use led2d;
#[cfg(not(feature = "host"))]
#[doc(hidden)] // Public for macro expansion in downstream crates; not a user-facing API.
pub use led2d_from_strip;
