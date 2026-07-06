use core::convert::Infallible;

use device_envoy_core::cyd::RegionPixels;
use embedded_graphics::{
    Pixel,
    pixelcolor::{IntoStorage, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Size},
};
use static_cell::StaticCell;

/// A fixed `WIDTH`x`HEIGHT` RGB565 pixel buffer, usable directly as an
/// [`embedded_graphics::draw_target::DrawTarget`].
pub struct RegionBuffer<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize> {
    pixels: [u16; PIXEL_COUNT],
}

/// A `PIXEL_COUNT`-sized RGB565 pixel workspace a [`super::CydEsp`] can own and
/// hand out [`RegionView`]s from, sized smaller than the full screen for
/// tiled drawing.
pub struct PixelBuffer<const PIXEL_COUNT: usize> {
    pixels: [u16; PIXEL_COUNT],
}

/// A borrowed `width`x`height` view into a [`PixelBuffer`].
pub struct RegionView<'a> {
    width: usize,
    height: usize,
    pixels: &'a mut [u16],
}

impl<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize>
    RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT>
{
    #[must_use]
    pub fn new() -> Self {
        assert!(
            PIXEL_COUNT == WIDTH * HEIGHT,
            "PIXEL_COUNT must equal WIDTH * HEIGHT"
        );
        Self {
            pixels: [0; PIXEL_COUNT],
        }
    }

    pub fn init_static(
        storage: &'static StaticCell<Self>,
    ) -> &'static mut RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT> {
        storage.init_with(Self::new)
    }

    pub fn fill(&mut self, color: Rgb565) {
        self.pixels.fill(color.into_storage());
    }

    #[must_use]
    pub fn raw_pixels(&self) -> &[u16; PIXEL_COUNT] {
        &self.pixels
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize> Default
    for RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize> RegionPixels
    for RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT>
{
    fn width(&self) -> usize {
        WIDTH
    }

    fn height(&self) -> usize {
        HEIGHT
    }

    fn raw_pixels(&self) -> &[u16] {
        &self.pixels
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize> DrawTarget
    for RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT>
{
    type Color = Rgb565;
    type Error = Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color);
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 {
                continue;
            }
            let point_x = point.x as usize;
            let point_y = point.y as usize;
            if point_x >= WIDTH || point_y >= HEIGHT {
                continue;
            }
            self.pixels[point_y * WIDTH + point_x] = color.into_storage();
        }
        Ok(())
    }
}

impl<const PIXEL_COUNT: usize> PixelBuffer<PIXEL_COUNT> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pixels: [0; PIXEL_COUNT],
        }
    }

    pub fn init_static(
        storage: &'static StaticCell<Self>,
    ) -> &'static mut PixelBuffer<PIXEL_COUNT> {
        storage.init_with(Self::new)
    }

    pub fn view_mut(&mut self, width: usize, height: usize) -> RegionView<'_> {
        let pixel_count = width * height;
        assert!(pixel_count <= PIXEL_COUNT, "view must fit in workspace");
        RegionView {
            width,
            height,
            pixels: &mut self.pixels[..pixel_count],
        }
    }
}

impl<const PIXEL_COUNT: usize> Default for PixelBuffer<PIXEL_COUNT> {
    fn default() -> Self {
        Self::new()
    }
}

// todo00 understand this code.
/// Type-erased draw buffer a [`CydEsp`](super::CydEsp) can own: it can be initialized
/// into a `'static` cell and hand out [`RegionView`]s. Implemented for every
/// [`PixelBuffer<PIXEL_COUNT>`] so that `CydEsp` can hold a buffer of any size without
/// itself being generic. Internal only — apps pick the size via the
/// `PIXEL_COUNT` on their [`CydStaticEsp`](super::CydStaticEsp).
pub(crate) trait DynPixelBuffer: 'static {
    /// Borrow a `width`×`height` view out of the buffer (must fit the capacity).
    fn view_mut(&mut self, width: usize, height: usize) -> RegionView<'_>;
}

impl<const PIXEL_COUNT: usize> DynPixelBuffer for PixelBuffer<PIXEL_COUNT> {
    fn view_mut(&mut self, width: usize, height: usize) -> RegionView<'_> {
        PixelBuffer::view_mut(self, width, height)
    }
}

impl RegionView<'_> {
    pub fn fill(&mut self, color: Rgb565) {
        self.pixels.fill(color.into_storage());
    }

    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.pixels
    }
}

impl RegionPixels for RegionView<'_> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn raw_pixels(&self) -> &[u16] {
        self.pixels
    }
}

impl DrawTarget for RegionView<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color);
        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 {
                continue;
            }
            let point_x = point.x as usize;
            let point_y = point.y as usize;
            if point_x >= self.width || point_y >= self.height {
                continue;
            }
            self.pixels[point_y * self.width + point_x] = color.into_storage();
        }
        Ok(())
    }
}

impl OriginDimensions for RegionView<'_> {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const PIXEL_COUNT: usize> OriginDimensions
    for RegionBuffer<WIDTH, HEIGHT, PIXEL_COUNT>
{
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}
