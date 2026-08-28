//! Compile-time image storage, conversion, masking, and drawing internals.

use crate::cyd::display::CydFrame;
use embedded_graphics::{
    Drawable, Pixel,
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::{DrawTarget, Point, Size},
    primitives::Rectangle,
};

/// Returns the storage length required by [`MaskFixed`] for an image size.
///
/// Each image pixel uses one bit, and the final byte includes any padding bits
/// needed when the pixel count is not divisible by eight.
///
/// Use this function for `MaskFixed`'s third const argument when calling
/// [`Image888Fixed::to_mask_magenta`]. See the visual
/// [`MaskFixed` example](MaskFixed).
pub const fn mask_byte_count(width: usize, height: usize) -> usize {
    (width * height).div_ceil(8)
}

/// A fixed-size RGB888 source image stored directly in the value.
///
/// `W` and `H` are the image dimensions and `N` is the pixel count (`W * H`).
/// Use [`tga!`](macro@crate::cyd::display::tga) to embed and decode an image
/// file at compile time, then convert it to display-ready RGB565 storage or a
/// visibility mask. See the visual [`MaskFixed` example](MaskFixed) for
/// color-key transparency.
///
/// # Example
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::{Image565Fixed, Image888Fixed, tga};
///
/// const SOURCE: Image888Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
///     env!("CARGO_MANIFEST_DIR"),
///     "/docs/assets/cyd_fill_contiguous.tga"
/// ));
/// const IMAGE: Image565Fixed<45, 73, { 45 * 73 }> = SOURCE.to_565();
///
/// assert_eq!(SOURCE.pixels.len(), 45 * 73);
/// assert_eq!(IMAGE.pixels.len(), 45 * 73);
/// ```
pub struct Image888Fixed<const W: usize, const H: usize, const N: usize> {
    /// Row-major, top-left-origin pixels stored as `[red, green, blue]`.
    pub pixels: [[u8; 3]; N],
}

/// A fixed-size RGB565 image stored directly in the value.
///
/// `W` and `H` are the image dimensions and `N` is the pixel count (`W * H`).
/// The image has no alpha channel. Use [`MaskFixed`] and
/// [`PlacedImage565::draw_masked`] when color-key transparency is needed.
/// The visual [`MaskFixed` example](MaskFixed) shows the opaque source and
/// masked result side by side. Convert a compile-time [`Image888Fixed`] with
/// [`Image888Fixed::to_565`].
///
/// Use [`Image565Fixed::view`] to borrow the complete image as an
/// [`Image565View`](super::Image565View), or [`Image565Fixed::view_rect`] to
/// borrow a crop without copying pixels.
///
/// # Example
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "image565_fixed",
        "docs/assets/image565_fixed.png"
    )
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::{
    UnwrapInfallible,
    cyd::{
        Cyd, CydDisplay,
        display::{CydFrame, Image565Fixed, tga},
    },
};
use embedded_graphics::{Drawable, prelude::Point};

const IMAGE: Image565Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/assets/cyd_fill_contiguous.tga"
))
.to_565();

async fn draw<C: Cyd>(cyd: &mut C) -> Result<(), C::Error> {
    let display = cyd.display();
    let mut frame = display.full_frame_mut();
    for top_left in [
        Point::new(50, 84),
        Point::new(138, 84),
        Point::new(226, 84),
    ] {
        IMAGE.at(top_left).draw(&mut frame).unwrap_infallible();
    }
    frame.flush().await
}

# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     pixelcolor::Rgb888,
#     prelude::{RgbColor, Size},
# };
# let mut cyd_memory = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# futures_executor::block_on(draw(&mut cyd_memory))?;
# let golden_result = assert_framebuffer_matches_expected_png(
#     &cyd_memory,
#     env!("CARGO_MANIFEST_DIR"),
#     "image565_fixed.png",
# );
# assert!(golden_result.is_ok(), "{golden_result:?}");
# Ok::<(), device_envoy_core::memory::Error>(())
```

![The same fixed RGB565 image drawn at three display positions][image565_fixed]
"#
)]
pub struct Image565Fixed<const W: usize, const H: usize, const N: usize> {
    /// Row-major, top-left-origin pixels packed as `RRRRR_GGGGGG_BBBBB`.
    pub pixels: [u16; N],
}

/// A packed binary visibility mask with one bit per image pixel.
///
/// Set bits are drawn and clear bits are transparent. Create a mask from an
/// [`Image888Fixed`] with [`Image888Fixed::to_mask_magenta`], then pass it to
/// [`PlacedImage565::draw_masked`].
///
/// # Example
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!("mask_fixed", "docs/assets/mask_fixed.png")
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::{
    UnwrapInfallible,
    cyd::{
        Cyd, CydDisplay,
        display::{
            CydFrame, Image565Fixed, Image888Fixed, MaskFixed, mask_byte_count, tga,
        },
    },
};
use embedded_graphics::{Drawable, prelude::Point};

const SOURCE: Image888Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/assets/cyd_fill_contiguous.tga"
));
const IMAGE: Image565Fixed<45, 73, { 45 * 73 }> = SOURCE.to_565();
const MASK: MaskFixed<45, 73, { mask_byte_count(45, 73) }> =
    SOURCE.to_mask_magenta();

async fn draw<C: Cyd>(cyd: &mut C) -> Result<(), C::Error> {
    let display = cyd.display();
    let mut frame = display.full_frame_mut();
    IMAGE
        .at(Point::new(80, 84))
        .draw(&mut frame)
        .unwrap_infallible();
    IMAGE
        .at(Point::new(200, 84))
        .draw_masked(&MASK, &mut frame)
        .unwrap_infallible();
    frame.flush().await
}

assert!(!MASK.is_set(0));
# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     pixelcolor::Rgb888,
#     prelude::{RgbColor, Size},
# };
# let mut cyd_memory = CydMemory::new(
#     Size::new(320, 240),
#     Rgb888::BLACK,
#     Rgb888::WHITE,
#     &FONT_9X15_BOLD,
# );
# futures_executor::block_on(draw(&mut cyd_memory))?;
# let golden_result = assert_framebuffer_matches_expected_png(
#     &cyd_memory,
#     env!("CARGO_MANIFEST_DIR"),
#     "mask_fixed.png",
# );
# assert!(golden_result.is_ok(), "{golden_result:?}");
# Ok::<(), device_envoy_core::memory::Error>(())
```

The opaque RGB565 image is on the left. The masked drawing on the right skips
the magenta background:

![An opaque RGB565 image beside the same image drawn with a transparency mask][mask_fixed]
"#
)]
pub struct MaskFixed<const W: usize, const H: usize, const MASK_N: usize> {
    /// Row-major visibility bits, least-significant bit first within each byte.
    pub bits: [u8; MASK_N],
}

/// A lightweight adapter that positions an [`Image565Fixed`] for drawing.
///
/// Drawing an `Image565Fixed` directly places its top-left pixel at `(0, 0)`.
/// [`Image565Fixed::at`] returns this adapter to place that pixel at another
/// display coordinate. It only stores a reference and a position; it does not
/// copy pixels, allocate storage, or enlarge the image.
///
/// This type is normally used as a temporary value rather than named or stored.
/// Call [`Drawable::draw`](https://docs.rs/embedded-graphics/latest/embedded_graphics/trait.Drawable.html)
/// to draw every pixel, or [`PlacedImage565::draw_masked`] to skip pixels that
/// are clear in a [`MaskFixed`]. The visual [`MaskFixed` example](MaskFixed)
/// shows the difference between those drawing modes.
///
/// # Example
///
/// Given a fixed image and its matching mask, position each drawing with
/// [`Image565Fixed::at`]:
///
/// ```rust,no_run
/// # use device_envoy_core::cyd::display::{
/// #     Image565Fixed, Image888Fixed, MaskFixed, mask_byte_count, tga,
/// # };
/// use embedded_graphics::{
///     Drawable,
///     draw_target::DrawTarget,
///     pixelcolor::Rgb565,
///     prelude::Point,
/// };
/// # const SOURCE: Image888Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
/// #     env!("CARGO_MANIFEST_DIR"),
/// #     "/docs/assets/cyd_fill_contiguous.tga"
/// # ));
/// # const IMAGE: Image565Fixed<45, 73, { 45 * 73 }> = SOURCE.to_565();
/// # const MASK: MaskFixed<45, 73, { mask_byte_count(45, 73) }> =
/// #     SOURCE.to_mask_magenta();
///
/// fn draw<D>(target: &mut D, top_left: Point) -> Result<(), D::Error>
/// where
///     D: DrawTarget<Color = Rgb565>,
/// {
///     IMAGE.at(top_left).draw(target)?;
///     IMAGE
///         .at(top_left + Point::new(60, 0))
///         .draw_masked(&MASK, target)
/// }
/// ```
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

impl<const W: usize, const H: usize, const N: usize> Image888Fixed<W, H, N> {
    /// Decodes a supported TGA at compile time, preserving RGB and discarding alpha.
    ///
    /// Prefer the public [`tga!`](macro@crate::cyd::display::tga) macro shown in
    /// its compiled example. This constructor powers that macro when raw TGA
    /// bytes are already available.
    ///
    /// Panics during const evaluation if `N != W * H` or the bytes do not
    /// contain a supported TGA with matching dimensions.
    pub const fn from_tga(bytes: &[u8]) -> Self {
        assert!(N == W * H, "Image888Fixed: N must equal W * H");
        let (pixel_start, bytes_per_pixel, top_origin) = parse_header(bytes, W, H);
        let mut pixels = [[0u8; 3]; N];
        let mut y = 0;
        while y < H {
            let mut x = 0;
            while x < W {
                let source_y = if top_origin { y } else { H - 1 - y };
                let offset = pixel_start + (source_y * W + x) * bytes_per_pixel;
                let red = bytes[offset + 2];
                let green = bytes[offset + 1];
                let blue = bytes[offset];
                pixels[y * W + x] = [red, green, blue];
                x += 1;
            }
            y += 1;
        }
        Self { pixels }
    }

    /// Converts this source image to RGB565.
    ///
    /// Each channel is reduced to the RGB565 bit depth. See the
    /// [`Image565Fixed` example](Image565Fixed).
    pub const fn to_565(&self) -> Image565Fixed<W, H, N> {
        let mut pixels = [0u16; N];
        let mut index = 0;
        while index < N {
            let [red, green, blue] = self.pixels[index];
            pixels[index] =
                ((red as u16 >> 3) << 11) | ((green as u16 >> 2) << 5) | (blue as u16 >> 3);
            index += 1;
        }
        Image565Fixed { pixels }
    }

    /// Derives a binary visibility mask using magenta as the transparent color.
    ///
    /// Pixels with red and blue at least `200` and green at most `60` become
    /// transparent; all other pixels remain visible. See the
    /// [`MaskFixed` example](MaskFixed).
    ///
    /// Panics during const evaluation if `MASK_N` does not equal
    /// [`mask_byte_count(W, H)`](mask_byte_count).
    pub const fn to_mask_magenta<const MASK_N: usize>(&self) -> MaskFixed<W, H, MASK_N> {
        assert!(
            MASK_N == mask_byte_count(W, H),
            "Mask: MASK_N must match image dimensions"
        );
        let mut bits = [0u8; MASK_N];
        let mut index = 0;
        while index < N {
            let pixel = self.pixels[index];
            let red = pixel[0];
            let green = pixel[1];
            let blue = pixel[2];
            if !(red >= 200 && blue >= 200 && green <= 60) {
                bits[index / 8] |= 1 << (index % 8);
            }
            index += 1;
        }
        MaskFixed { bits }
    }
}

impl<const W: usize, const H: usize, const N: usize> Image565Fixed<W, H, N> {
    /// Returns a lightweight adapter that draws this image at `top_left`.
    ///
    /// Drawing the image directly places it at [`Point::zero`]. This method
    /// changes that drawing position without copying or modifying the image.
    /// The returned [`PlacedImage565`] supports ordinary and masked drawing;
    /// see its [compact example](PlacedImage565).
    pub const fn at(&self, top_left: Point) -> PlacedImage565<'_, W, H, N> {
        PlacedImage565 {
            image: self,
            top_left,
        }
    }

    /// View the complete image as RGB565 pixels.
    ///
    /// This is a zero-copy view over the complete image. See the
    /// [`Image565View` example](super::Image565View).
    pub const fn view(&'static self) -> super::Image565View {
        self.view_rect(Rectangle::new(Point::zero(), Size::new(W as u32, H as u32)))
    }

    /// View a validated rectangular crop of the image without copying pixels.
    ///
    /// `source` uses coordinates in the full image. Coordinates used through
    /// the returned view are local to that rectangle. See the
    /// [`Image565View` example](super::Image565View).
    ///
    /// Panics if `source` has a negative origin or extends outside the image.
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

    /// Bulk-copies the complete image into a frame with matching dimensions.
    ///
    /// This uses [`CydFrame::copy_from_565`] and does not flush the frame.
    /// Returns [`crate::Error::CopySize`] if the image and frame pixel counts
    /// differ.
    ///
    /// ```rust,no_run
    /// use device_envoy_core::cyd::display::{CydFrame, Image565Fixed};
    ///
    /// fn copy<const W: usize, const H: usize, const N: usize, F: CydFrame>(
    ///     image: &Image565Fixed<W, H, N>,
    ///     frame: &mut F,
    /// ) -> device_envoy_core::Result<()> {
    ///     image.copy_to(frame)
    /// }
    /// ```
    pub fn copy_to<F: CydFrame>(&self, frame: &mut F) -> crate::Result<()> {
        frame.copy_from_565(&self.pixels)
    }
}

impl<const W: usize, const H: usize, const MASK_N: usize> MaskFixed<W, H, MASK_N> {
    /// Returns whether the row-major pixel at `index` is visible.
    ///
    /// See the [`MaskFixed` example](MaskFixed).
    pub const fn is_set(&self, index: usize) -> bool {
        self.bits[index / 8] & (1 << (index % 8)) != 0
    }
}

impl<'a, const W: usize, const H: usize, const N: usize> PlacedImage565<'a, W, H, N> {
    /// Draws the positioned image, skipping pixels that are clear in `mask`.
    ///
    /// [`Image565Fixed::at`] determines the display position; the mask only
    /// determines which pixels within the image are drawn. The image and mask
    /// dimensions must match. See the visual [`MaskFixed` example](MaskFixed).
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
        target.draw_iter(core::iter::from_fn(|| {
            loop {
                let pixel = pixels.next()?;
                let index = pixels.index - 1;
                if mask.is_set(index) {
                    return Some(pixel);
                }
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
    /// Draws every image pixel at the display position supplied to
    /// [`Image565Fixed::at`].
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
        $crate::cyd::display::Image888Fixed::from_tga(include_bytes!($path))
    };
    ($path:expr, $width:expr, $height:expr) => {
        $crate::cyd::display::Image888Fixed::<$width, $height, { $width * $height }>::from_tga(
            include_bytes!($path),
        )
    };
}
