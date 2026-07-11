//! Compile-time decoding and drawing of the supported subset of TGA images.

use crate::cyd::display::CydFrame;
use crate::pixel_target::rgb565_raw_from_rgb888_components;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
    Drawable, Pixel,
};

/// Returns the number of bytes required for a packed one-bit mask.
pub const fn mask_byte_count(width: usize, height: usize) -> usize {
    (width * height + 7) / 8
}

/// A decoded, row-major RGBA8888 image. Pixels are packed as `0xRRGGBBAA`.
pub struct TgaImageFixed<const W: usize, const H: usize, const N: usize> {
    /// Row-major top-left-origin RGBA8888 pixels.
    pub pixels: [u32; N],
}

/// An opaque RGB565 image.
pub struct Image565Fixed<const W: usize, const H: usize, const N: usize> {
    /// Row-major top-left-origin pixels.
    pub pixels: [u16; N],
}

/// A packed binary visibility mask with one bit per image pixel.
pub struct MaskFixed<const W: usize, const H: usize, const MASK_N: usize> {
    /// Row-major bits, least-significant bit first. Set means visible.
    pub bits: [u8; MASK_N],
}

pub struct PlacedImage565<'a, const W: usize, const H: usize, const N: usize> {
    image: &'a Image565Fixed<W, H, N>,
    top_left: Point,
}

const fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    bytes[offset] as u16 | ((bytes[offset + 1] as u16) << 8)
}

const fn parse_header(bytes: &[u8], width: usize, height: usize) -> (usize, usize, bool) {
    assert!(bytes.len() >= 18, "TGA: file shorter than header");
    assert!(bytes[1] == 0, "TGA: color maps are not supported");
    assert!(
        bytes[2] == 2,
        "TGA: only uncompressed true-color images are supported"
    );
    assert!(
        bytes[3] == 0 && bytes[4] == 0 && bytes[5] == 0 && bytes[6] == 0 && bytes[7] == 0,
        "TGA: color map specification is not supported"
    );
    assert!(
        read_u16(bytes, 12) as usize == width,
        "TGA: width does not match const argument"
    );
    assert!(
        read_u16(bytes, 14) as usize == height,
        "TGA: height does not match const argument"
    );
    assert!(
        bytes[16] == 24 || bytes[16] == 32,
        "TGA: only 24-bit BGR or 32-bit BGRA is supported"
    );
    assert!(
        bytes[17] & 0x10 == 0,
        "TGA: right-to-left origin is not supported"
    );
    let bytes_per_pixel = (bytes[16] / 8) as usize;
    let pixel_start = 18 + bytes[0] as usize;
    assert!(
        bytes.len() >= pixel_start + width * height * bytes_per_pixel,
        "TGA: pixel data is shorter than width * height"
    );
    (pixel_start, bytes_per_pixel, bytes[17] & 0x20 != 0)
}

const fn source_offset(
    pixel_start: usize,
    bytes_per_pixel: usize,
    top_origin: bool,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) -> usize {
    let source_y = if top_origin { y } else { height - 1 - y };
    pixel_start + (source_y * width + x) * bytes_per_pixel
}

impl<const W: usize, const H: usize, const N: usize> TgaImageFixed<W, H, N> {
    /// Decodes a supported TGA at compile time, preserving RGB and alpha.
    pub const fn from_tga(bytes: &[u8]) -> Self {
        assert!(N == W * H, "TgaImage: N must equal W * H");
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);
        let mut pixels = [0u32; N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let offset = source_offset(pixel_start, bytes_per_pixel, top_origin, W, H, x, y);
                let red = bytes[offset + 2] as u32;
                let green = bytes[offset + 1] as u32;
                let blue = bytes[offset] as u32;
                let alpha = if bytes_per_pixel == 4 {
                    bytes[offset + 3]
                } else {
                    255
                } as u32;
                pixels[y * W + x] = (red << 24) | (green << 16) | (blue << 8) | alpha;
                x += 1;
            }
            y += 1;
        }
        Self { pixels }
    }

    /// Converts this source image to RGB565.
    pub const fn to_565(&self) -> Image565Fixed<W, H, N> {
        let mut pixels = [0u16; N];
        let mut index = 0;
        while index < N {
            let pixel = self.pixels[index];
            pixels[index] = rgb565_raw_from_rgb888_components(
                (pixel >> 24) as u8,
                (pixel >> 16) as u8,
                (pixel >> 8) as u8,
            );
            index += 1;
        }
        Image565Fixed { pixels }
    }

    /// Derives a binary mask using the magenta color key.
    pub const fn to_mask_magenta<const MASK_N: usize>(&self) -> MaskFixed<W, H, MASK_N> {
        assert!(
            MASK_N == mask_byte_count(W, H),
            "Mask: MASK_N must match image dimensions"
        );
        let mut bits = [0u8; MASK_N];
        let mut index = 0;
        while index < N {
            let pixel = self.pixels[index];
            let red = (pixel >> 24) as u8;
            let green = (pixel >> 16) as u8;
            let blue = (pixel >> 8) as u8;
            if !(red >= 200 && blue >= 200 && green <= 60) {
                bits[index / 8] |= 1 << (index % 8);
            }
            index += 1;
        }
        MaskFixed { bits }
    }
}

impl<const W: usize, const H: usize, const N: usize> Image565Fixed<W, H, N> {
    pub const fn at(&self, top_left: Point) -> PlacedImage565<'_, W, H, N> {
        PlacedImage565 {
            image: self,
            top_left,
        }
    }

    pub const fn view(&'static self) -> super::Image565View {
        self.view_rect(Rectangle::new(Point::zero(), Size::new(W as u32, H as u32)))
    }

    pub const fn view_rect(&'static self, source: Rectangle) -> super::Image565View {
        assert!(
            source.top_left.x >= 0 && source.top_left.y >= 0,
            "view_rect: negative origin"
        );
        assert!(
            source.top_left.x as usize + source.size.width as usize <= W
                && source.top_left.y as usize + source.size.height as usize <= H,
            "view_rect: rectangle is outside image"
        );
        super::Image565View::new_cropped(&self.pixels, W as u32, source)
    }

    pub fn copy_to<F: CydFrame>(&self, frame: &mut F) -> crate::Result<()> {
        frame.copy_from_565(&self.pixels)
    }

    pub fn rgb565_iter(&self) -> impl Iterator<Item = Rgb565> + '_ {
        self.pixels
            .iter()
            .copied()
            .map(|pixel| Rgb565::from(RawU16::new(pixel)))
    }
}

impl<const W: usize, const H: usize, const MASK_N: usize> MaskFixed<W, H, MASK_N> {
    pub const fn is_set(&self, index: usize) -> bool {
        self.bits[index / 8] & (1 << (index % 8)) != 0
    }
}

impl<'a, const W: usize, const H: usize, const N: usize> PlacedImage565<'a, W, H, N> {
    pub fn draw_masked<const MASK_N: usize, D>(
        &self,
        mask: &MaskFixed<W, H, MASK_N>,
        target: &mut D,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let mut pixels = ImagePixels::<W, N> {
            pixels: &self.image.pixels,
            top_left: self.top_left,
            index: 0,
        };
        target.draw_iter(core::iter::from_fn(|| loop {
            let pixel = pixels.next()?;
            let index = pixels.index - 1;
            if mask.is_set(index) {
                return Some(pixel);
            }
        }))
    }
}

struct ImagePixels<'a, const W: usize, const N: usize> {
    pixels: &'a [u16; N],
    top_left: Point,
    index: usize,
}

impl<const W: usize, const N: usize> Iterator for ImagePixels<'_, W, N> {
    type Item = Pixel<Rgb565>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= N {
            return None;
        }
        let index = self.index;
        self.index += 1;
        Some(Pixel(
            self.top_left + Point::new((index % W) as i32, (index / W) as i32),
            Rgb565::from(RawU16::new(self.pixels[index])),
        ))
    }
}

impl<const W: usize, const H: usize, const N: usize> Drawable for PlacedImage565<'_, W, H, N> {
    type Color = Rgb565;
    type Output = ();
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        target.draw_iter(ImagePixels::<W, N> {
            pixels: &self.image.pixels,
            top_left: self.top_left,
            index: 0,
        })
    }
}

impl<const W: usize, const H: usize, const N: usize> Drawable for Image565Fixed<W, H, N> {
    type Color = Rgb565;
    type Output = ();
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.at(Point::zero()).draw(target)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __cyd_tga {
    ($path:expr) => {
        $crate::cyd::display::TgaImageFixed::<_, _, _>::from_tga(include_bytes!($path))
    };
    ($path:expr, $width:expr, $height:expr) => {
        $crate::cyd::display::TgaImageFixed::<$width, $height, { $width * $height }>::from_tga(
            include_bytes!($path),
        )
    };
}
