//! Compile-time TGA decoding into RGB565 images.
//!
//! [`Image565Fixed`] (opaque) and [`Image565Mask`] (with a binary transparency mask)
//! are decoded from an embedded `.tga` byte slice entirely in `const fn`, so the
//! pixels live in read-only flash with no runtime allocation or parsing. Use the
//! [`tga565!`](crate::cyd::tga565) and [`tga565_mask!`](crate::cyd::tga565_mask)
//! macros to avoid spelling the `N`/`MASK_N` const arguments by hand.
//!
//! # Accepted subset (TGA565 v1)
//!
//! - uncompressed true-color TGA only (image type `2`)
//! - no color map (color map type `0`)
//! - 24-bit BGR or 32-bit BGRA
//! - width/height must match the const-generic arguments
//! - top-left or bottom-left origin supported
//! - right-to-left origin rejected
//! - 32-bit alpha is binary: `0` transparent, nonzero opaque
//! - no RLE, no palettes, no grayscale
//!
//! Anything outside this subset triggers a `const` panic, so an unsupported file
//! fails the build rather than at runtime.

use crate::pixel_target::rgb565_raw_from_rgb888_components;
use embedded_graphics::{
    Drawable, Pixel,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};

/// An opaque RGB565 image decoded from a TGA at compile time.
///
/// `N` must equal `W * H`; the [`tga565!`](crate::cyd::tga565) macro computes it for
/// you. 32-bit (BGRA) sources are accepted but their alpha is ignored — use
/// [`Image565Mask`] when transparency matters.
pub struct Image565Fixed<const W: usize, const H: usize, const N: usize> {
    /// Row-major top-left-origin pixels, one RGB565 value each.
    pub pixels: [u16; N],
}

/// An [`Image565Fixed`] placed at a concrete target position.
pub struct PlacedImage565<'a, const W: usize, const H: usize, const N: usize> {
    image: &'a Image565Fixed<W, H, N>,
    top_left: Point,
}

/// An RGB565 image with a 1-bit-per-pixel opacity mask, decoded from a 24- or
/// 32-bit TGA at compile time.
///
/// `N` must equal `W * H` and `MASK_N` must equal `(W * H + 7) / 8`; the
/// [`tga565_mask!`](crate::cyd::tga565_mask) macro computes both. For 24-bit sources
/// every pixel is opaque; for 32-bit sources a pixel is transparent exactly when
/// its alpha byte is `0`.
pub struct Image565Mask<const W: usize, const H: usize, const N: usize, const MASK_N: usize> {
    /// Row-major top-left-origin pixels, one RGB565 value each.
    pub pixels: [u16; N],
    /// 1 bit per pixel, row-major: set means opaque. Bit `i` is byte `i / 8`,
    /// bit `i % 8` (LSB first).
    pub opaque: [u8; MASK_N],
}

/// An [`Image565Mask`] placed at a concrete target position.
pub struct PlacedImage565Mask<
    'a,
    const W: usize,
    const H: usize,
    const N: usize,
    const MASK_N: usize,
> {
    image: &'a Image565Mask<W, H, N, MASK_N>,
    top_left: Point,
}

/// Little-endian `u16` read at `offset`.
const fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    (bytes[offset] as u16) | ((bytes[offset + 1] as u16) << 8)
}

/// Validates the 18-byte header against the v1 subset and returns
/// `(pixel_start, bytes_per_pixel, top_origin)`.
const fn parse_header(bytes: &[u8], width: usize, height: usize) -> (usize, usize, bool) {
    assert!(
        bytes.len() >= 18,
        "TGA: file shorter than its 18-byte header"
    );

    let color_map_type = bytes[1];
    assert!(
        color_map_type == 0,
        "TGA: color-mapped images are not supported"
    );

    let image_type = bytes[2];
    assert!(
        image_type == 2,
        "TGA: only uncompressed true-color (type 2) is supported"
    );

    // Color map specification (bytes 3..=7) must be all zero for this subset.
    assert!(
        bytes[3] == 0 && bytes[4] == 0 && bytes[5] == 0 && bytes[6] == 0 && bytes[7] == 0,
        "TGA: nonzero color map specification is not supported"
    );

    let file_width = read_u16(bytes, 12) as usize;
    let file_height = read_u16(bytes, 14) as usize;
    assert!(
        file_width == width,
        "TGA: width does not match const argument"
    );
    assert!(
        file_height == height,
        "TGA: height does not match const argument"
    );

    let pixel_depth = bytes[16];
    assert!(
        pixel_depth == 24 || pixel_depth == 32,
        "TGA: only 24-bit BGR or 32-bit BGRA is supported"
    );
    let bytes_per_pixel = (pixel_depth / 8) as usize;

    let descriptor = bytes[17];
    assert!(
        descriptor & 0x10 == 0,
        "TGA: right-to-left origin is not supported"
    );
    let top_origin = descriptor & 0x20 != 0;

    let id_length = bytes[0] as usize;
    let pixel_start = 18 + id_length;
    assert!(
        bytes.len() >= pixel_start + width * height * bytes_per_pixel,
        "TGA: pixel data is shorter than width * height"
    );

    (pixel_start, bytes_per_pixel, top_origin)
}

/// Source byte offset of pixel `(x, y)` in the top-left-origin output, given the
/// decoded header.
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

impl<const W: usize, const H: usize, const N: usize> Image565Fixed<W, H, N> {
    /// Decodes `bytes` (an embedded `.tga`) into an opaque RGB565 image.
    ///
    /// Panics at compile time if `N != W * H` or if the file falls outside the
    /// accepted subset. Prefer [`tga565!`](crate::cyd::tga565) at call sites.
    pub const fn from_tga(bytes: &[u8]) -> Self {
        assert!(N == W * H, "Image565: N must equal W * H");
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);

        let mut pixels = [0u16; N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let offset = source_offset(pixel_start, bytes_per_pixel, top_origin, W, H, x, y);
                let blue = bytes[offset];
                let green = bytes[offset + 1];
                let red = bytes[offset + 2];
                pixels[y * W + x] = rgb565_raw_from_rgb888_components(red, green, blue);
                x += 1;
            }
            y += 1;
        }

        Self { pixels }
    }

    /// Bulk-copy this full-frame image into `frame` via
    /// [`CydFrame::copy_from_565`](super::CydFrame::copy_from_565) — the fast path for a full-screen
    /// background. Much cheaper than the per-pixel [`Drawable`] path; returns
    /// [`CopySizeError`](super::CopySizeError) if the image's dimensions don't match the frame's.
    pub fn copy_to<F: super::CydFrame>(&self, frame: &mut F) -> Result<(), super::CopySizeError> {
        frame.copy_from_565(&self.pixels)
    }

    /// Return a drawable image with its top-left corner at `top_left`.
    #[must_use]
    pub const fn at(&self, top_left: Point) -> PlacedImage565<'_, W, H, N> {
        PlacedImage565 {
            image: self,
            top_left,
        }
    }

    /// Full-image view of this bitmap.
    #[must_use]
    pub const fn view(&'static self) -> super::Image565View {
        super::Image565View::new_cropped(
            &self.pixels,
            W as u32,
            Rectangle::new(Point::zero(), Size::new(W as u32, H as u32)),
        )
    }

    /// Cropped view of this bitmap. `source` is in image coordinates.
    ///
    /// Panics if `source` extends outside the image bounds.
    #[must_use]
    pub const fn view_rect(&'static self, source: Rectangle) -> super::Image565View {
        assert!(
            source.top_left.x >= 0 && source.top_left.y >= 0,
            "view_rect: source top-left must be non-negative"
        );
        assert!(
            source.top_left.x as usize + source.size.width as usize <= W
                && source.top_left.y as usize + source.size.height as usize <= H,
            "view_rect: source rectangle must be inside the image"
        );
        super::Image565View::new_cropped(&self.pixels, W as u32, source)
    }

    /// Iterate the pixels as [`Rgb565`] values, row-major, top-left first.
    pub fn rgb565_iter(&self) -> impl Iterator<Item = Rgb565> + '_ {
        self.pixels
            .iter()
            .copied()
            .map(|p| Rgb565::from(RawU16::new(p)))
    }
}

impl<const W: usize, const H: usize, const N: usize> Drawable for PlacedImage565<'_, W, H, N> {
    type Color = Rgb565;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        target.draw_iter(Image565Pixels {
            image: self.image,
            top_left: self.top_left,
            index: 0,
        })
    }
}

impl<const W: usize, const H: usize, const N: usize> Drawable for Image565Fixed<W, H, N> {
    type Color = Rgb565;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.at(Point::new(0, 0)).draw(target)
    }
}

struct Image565Pixels<'a, const W: usize, const H: usize, const N: usize> {
    image: &'a Image565Fixed<W, H, N>,
    top_left: Point,
    index: usize,
}

impl<const W: usize, const H: usize, const N: usize> Iterator for Image565Pixels<'_, W, H, N> {
    type Item = Pixel<Rgb565>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= N {
            return None;
        }

        let index = self.index;
        self.index += 1;
        let x = index % W;
        let y = index / W;
        Some(Pixel(
            self.top_left + Point::new(x as i32, y as i32),
            Rgb565::from(RawU16::new(self.image.pixels[index])),
        ))
    }
}

impl<const W: usize, const H: usize, const N: usize, const MASK_N: usize>
    Image565Mask<W, H, N, MASK_N>
{
    /// Decodes `bytes` (an embedded `.tga`) into an RGB565 image plus a binary
    /// transparency mask.
    ///
    /// Panics at compile time if `N != W * H`, if `MASK_N != (W * H + 7) / 8`,
    /// or if the file falls outside the accepted subset. Prefer
    /// [`tga565_mask!`](crate::cyd::tga565_mask) at call sites.
    pub const fn from_tga(bytes: &[u8]) -> Self {
        assert!(N == W * H, "Image565Mask: N must equal W * H");
        assert!(
            MASK_N == (W * H + 7) / 8,
            "Image565Mask: MASK_N must equal (W * H + 7) / 8"
        );
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);

        let mut pixels = [0u16; N];
        let mut opaque = [0u8; MASK_N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let offset = source_offset(pixel_start, bytes_per_pixel, top_origin, W, H, x, y);
                let blue = bytes[offset];
                let green = bytes[offset + 1];
                let red = bytes[offset + 2];
                let index = y * W + x;
                pixels[index] = rgb565_raw_from_rgb888_components(red, green, blue);
                // 24-bit has no alpha and is fully opaque; 32-bit uses binary
                // alpha (0 transparent, nonzero opaque).
                let is_opaque = bytes_per_pixel == 3 || bytes[offset + 3] != 0;
                if is_opaque {
                    opaque[index / 8] |= 1 << (index % 8);
                }
                x += 1;
            }
            y += 1;
        }

        Self { pixels, opaque }
    }

    /// Decodes `bytes` (an embedded `.tga`) into an RGB565 image, treating
    /// magenta (full red, full blue, near-zero green) as the transparent
    /// color-key instead of using the alpha channel. Useful for art exported
    /// without alpha, where a magenta backdrop marks the transparent areas.
    ///
    /// A pixel is transparent when `red >= 200 && blue >= 200 && green <= 60`,
    /// which also catches the anti-aliased fringe of the key color. Panics at
    /// compile time under the same conditions as [`from_tga`](Self::from_tga).
    pub const fn from_tga_magenta(bytes: &[u8]) -> Self {
        assert!(N == W * H, "Image565Mask: N must equal W * H");
        assert!(
            MASK_N == (W * H + 7) / 8,
            "Image565Mask: MASK_N must equal (W * H + 7) / 8"
        );
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);

        let mut pixels = [0u16; N];
        let mut opaque = [0u8; MASK_N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let offset = source_offset(pixel_start, bytes_per_pixel, top_origin, W, H, x, y);
                let blue = bytes[offset];
                let green = bytes[offset + 1];
                let red = bytes[offset + 2];
                let index = y * W + x;
                pixels[index] = rgb565_raw_from_rgb888_components(red, green, blue);
                // Magenta color-key: transparent where red and blue are high and
                // green is low (covers the anti-aliased fringe of pure magenta).
                let is_magenta = red >= 200 && blue >= 200 && green <= 60;
                if !is_magenta {
                    opaque[index / 8] |= 1 << (index % 8);
                }
                x += 1;
            }
            y += 1;
        }

        Self { pixels, opaque }
    }

    /// Decodes `bytes` (an embedded `.tga`) into an RGB565 image, treating
    /// white (all channels near full) as the transparent color-key instead of
    /// using the alpha channel. Useful for art exported on a white backdrop with
    /// an unused (all-opaque) alpha channel.
    ///
    /// A pixel is transparent when `red >= 220 && green >= 220 && blue >= 220`,
    /// which also catches the anti-aliased fringe of the white key. Panics at
    /// compile time under the same conditions as [`from_tga`](Self::from_tga).
    pub const fn from_tga_white(bytes: &[u8]) -> Self {
        assert!(N == W * H, "Image565Mask: N must equal W * H");
        assert!(
            MASK_N == (W * H + 7) / 8,
            "Image565Mask: MASK_N must equal (W * H + 7) / 8"
        );
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);

        let mut pixels = [0u16; N];
        let mut opaque = [0u8; MASK_N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let offset = source_offset(pixel_start, bytes_per_pixel, top_origin, W, H, x, y);
                let blue = bytes[offset];
                let green = bytes[offset + 1];
                let red = bytes[offset + 2];
                let index = y * W + x;
                pixels[index] = rgb565_raw_from_rgb888_components(red, green, blue);
                // White color-key: transparent where all channels are near full
                // (covers the anti-aliased fringe of pure white).
                let is_white = red >= 220 && green >= 220 && blue >= 220;
                if !is_white {
                    opaque[index / 8] |= 1 << (index % 8);
                }
                x += 1;
            }
            y += 1;
        }

        Self { pixels, opaque }
    }

    /// Returns whether pixel `index` (row-major) is opaque.
    pub const fn is_opaque(&self, index: usize) -> bool {
        self.opaque[index / 8] & (1 << (index % 8)) != 0
    }

    /// Return a drawable image with its top-left corner at `top_left`.
    #[must_use]
    pub const fn at(&self, top_left: Point) -> PlacedImage565Mask<'_, W, H, N, MASK_N> {
        PlacedImage565Mask {
            image: self,
            top_left,
        }
    }
}

impl<const W: usize, const H: usize, const N: usize, const MASK_N: usize> Drawable
    for PlacedImage565Mask<'_, W, H, N, MASK_N>
{
    type Color = Rgb565;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        target.draw_iter(Image565MaskPixels {
            image: self.image,
            top_left: self.top_left,
            index: 0,
        })
    }
}

impl<const W: usize, const H: usize, const N: usize, const MASK_N: usize> Drawable
    for Image565Mask<W, H, N, MASK_N>
{
    type Color = Rgb565;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.at(Point::new(0, 0)).draw(target)
    }
}

struct Image565MaskPixels<'a, const W: usize, const H: usize, const N: usize, const MASK_N: usize> {
    image: &'a Image565Mask<W, H, N, MASK_N>,
    top_left: Point,
    index: usize,
}

impl<const W: usize, const H: usize, const N: usize, const MASK_N: usize> Iterator
    for Image565MaskPixels<'_, W, H, N, MASK_N>
{
    type Item = Pixel<Rgb565>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < N {
            let index = self.index;
            self.index += 1;
            if self.image.is_opaque(index) {
                let x = index % W;
                let y = index / W;
                return Some(Pixel(
                    self.top_left + Point::new(x as i32, y as i32),
                    Rgb565::from(RawU16::new(self.image.pixels[index])),
                ));
            }
        }

        None
    }
}

/// Decodes an embedded `.tga` into an [`Image565Fixed`], computing the `N = W * H`
/// const argument for you.
///
/// ```rust,ignore
/// // `ignore`: `include_bytes!` resolves at compile time even under `no_run`,
/// // so this needs a real `dial.tga` on disk to build.
/// # use device_envoy_core::cyd::{Image565Fixed, tga565};
/// const DIAL: Image565Fixed<240, 276, { 240 * 276 }> =
///     tga565!("../assets/dial.tga", 240, 276);
/// ```
// Public macro helpers must originate at crate root for downstream expansion,
// but the user-facing names are re-exported only from `crate::cyd`.
#[doc(hidden)]
#[macro_export]
macro_rules! __cyd_tga565 {
    ($path:expr, $width:expr, $height:expr) => {
        $crate::cyd::Image565Fixed::<$width, $height, { $width * $height }>::from_tga(
            include_bytes!($path),
        )
    };
}

/// Decodes an embedded `.tga` into an [`Image565Mask`], computing the
/// `N = W * H` and `MASK_N = (W * H + 7) / 8` const arguments for you.
///
/// ```rust,ignore
/// // `ignore`: `include_bytes!` resolves at compile time even under `no_run`,
/// // so this needs a real `hour_sign.tga` on disk to build.
/// # use device_envoy_core::cyd::{Image565Mask, tga565_mask};
/// const HOUR_SIGN: Image565Mask<48, 32, { 48 * 32 }, { (48 * 32 + 7) / 8 }> =
///     tga565_mask!("../assets/hour_sign.tga", 48, 32);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __cyd_tga565_mask {
    ($path:expr, $width:expr, $height:expr) => {{
        type Image = $crate::cyd::Image565Mask<
            $width,
            $height,
            { $width * $height },
            { ($width * $height + 7) / 8 },
        >;
        Image::from_tga(include_bytes!($path))
    }};
}

/// Decodes an embedded `.tga` into an [`Image565Mask`] using magenta as the
/// transparent color-key (see [`Image565Mask::from_tga_magenta`]), computing the
/// `N = W * H` and `MASK_N = (W * H + 7) / 8` const arguments for you.
///
/// ```rust,ignore
/// # use device_envoy_core::cyd::{Image565Mask, tga565_magenta_mask};
/// const HOUR_SIGN: Image565Mask<34, 46, { 34 * 46 }, { (34 * 46 + 7) / 8 }> =
///     tga565_magenta_mask!("../assets/hours.small.tga", 34, 46);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __cyd_tga565_magenta_mask {
    ($path:expr, $width:expr, $height:expr) => {{
        type Image = $crate::cyd::Image565Mask<
            $width,
            $height,
            { $width * $height },
            { ($width * $height + 7) / 8 },
        >;
        Image::from_tga_magenta(include_bytes!($path))
    }};
}

/// Decodes an embedded `.tga` into an [`Image565Mask`] using white as the
/// transparent color-key (see [`Image565Mask::from_tga_white`]), computing the
/// `N = W * H` and `MASK_N = (W * H + 7) / 8` const arguments for you.
///
/// ```rust,ignore
/// # use device_envoy_core::cyd::{Image565Mask, tga565_white_mask};
/// const HOUR_SIGN: Image565Mask<45, 73, { 45 * 73 }, { (45 * 73 + 7) / 8 }> =
///     tga565_white_mask!("../assets/hours.small.tga", 45, 73);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __cyd_tga565_white_mask {
    ($path:expr, $width:expr, $height:expr) => {{
        type Image = $crate::cyd::Image565Mask<
            $width,
            $height,
            { $width * $height },
            { ($width * $height + 7) / 8 },
        >;
        Image::from_tga_white(include_bytes!($path))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an 18-byte true-color header for `width`x`height` at `depth` bits,
    /// with `top_origin` controlling the vertical-origin descriptor bit.
    fn header(width: u16, height: u16, depth: u8, top_origin: bool) -> [u8; 18] {
        let mut bytes = [0u8; 18];
        bytes[2] = 2; // uncompressed true-color
        bytes[12] = width as u8;
        bytes[13] = (width >> 8) as u8;
        bytes[14] = height as u8;
        bytes[15] = (height >> 8) as u8;
        bytes[16] = depth;
        bytes[17] = if top_origin { 0x20 } else { 0 };
        bytes
    }

    #[test]
    fn decodes_24bit_top_origin() {
        // 2x1, top-left origin: pure red then pure blue, stored as BGR.
        let mut bytes = header(2, 1, 24, true).to_vec();
        bytes.extend_from_slice(&[0x00, 0x00, 0xff]); // red
        bytes.extend_from_slice(&[0xff, 0x00, 0x00]); // blue
        let image = Image565Fixed::<2, 1, 2>::from_tga(&bytes);
        assert_eq!(
            image.pixels,
            [
                rgb565_raw_from_rgb888_components(0xff, 0, 0),
                rgb565_raw_from_rgb888_components(0, 0, 0xff)
            ]
        );
    }

    #[test]
    fn bottom_origin_rows_are_flipped() {
        // 1x2 bottom-up: file row 0 is the screen's bottom row.
        let mut bytes = header(1, 2, 24, false).to_vec();
        bytes.extend_from_slice(&[0x00, 0x00, 0xff]); // file row 0 -> output row 1
        bytes.extend_from_slice(&[0xff, 0x00, 0x00]); // file row 1 -> output row 0
        let image = Image565Fixed::<1, 2, 2>::from_tga(&bytes);
        assert_eq!(
            image.pixels,
            [
                rgb565_raw_from_rgb888_components(0, 0, 0xff),
                rgb565_raw_from_rgb888_components(0xff, 0, 0)
            ]
        );
    }

    #[test]
    fn mask_uses_binary_alpha() {
        // 2x1, 32-bit BGRA top origin: first pixel transparent, second opaque.
        let mut bytes = header(2, 1, 32, true).to_vec();
        bytes.extend_from_slice(&[0x00, 0x00, 0xff, 0x00]); // alpha 0 -> transparent
        bytes.extend_from_slice(&[0xff, 0x00, 0x00, 0x7f]); // alpha != 0 -> opaque
        let image = Image565Mask::<2, 1, 2, 1>::from_tga(&bytes);
        assert!(!image.is_opaque(0));
        assert!(image.is_opaque(1));
    }

    #[test]
    fn mask_24bit_is_fully_opaque() {
        let mut bytes = header(1, 1, 24, true).to_vec();
        bytes.extend_from_slice(&[0x10, 0x20, 0x30]);
        let image = Image565Mask::<1, 1, 1, 1>::from_tga(&bytes);
        assert!(image.is_opaque(0));
    }

    #[test]
    #[should_panic(expected = "width does not match")]
    fn rejects_wrong_width() {
        let mut bytes = header(3, 1, 24, true).to_vec();
        bytes.extend_from_slice(&[0u8; 9]);
        let _ = Image565Fixed::<2, 1, 2>::from_tga(&bytes);
    }

    #[test]
    #[should_panic(expected = "right-to-left")]
    fn rejects_right_to_left_origin() {
        let mut bytes = header(1, 1, 24, true).to_vec();
        bytes[17] |= 0x10;
        bytes.extend_from_slice(&[0u8; 3]);
        let _ = Image565Fixed::<1, 1, 1>::from_tga(&bytes);
    }
}
