use core::convert::Infallible;

use embedded_graphics::{
    Pixel,
    pixelcolor::{IntoStorage, Rgb565},
    prelude::{DrawTarget, OriginDimensions, Size},
};
use static_cell::StaticCell;

/// A `PIXEL_COUNT`-sized RGB565 pixel workspace a [`super::CydEsp`] can own and
/// hand out [`RegionView`]s from, sized smaller than the full screen for
/// tiled drawing.
pub(crate) struct PixelBuffer<const PIXEL_COUNT: usize> {
    pixels: [u16; PIXEL_COUNT],
}

/// A borrowed `width`×`height` view into a [`PixelBuffer`].
pub(crate) struct RegionView<'a> {
    width: usize,
    height: usize,
    pixels: &'a mut [u16],
}

impl<const PIXEL_COUNT: usize> PixelBuffer<PIXEL_COUNT> {
    /// Create a zeroed, `PIXEL_COUNT`-sized pixel workspace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pixels: [0; PIXEL_COUNT],
        }
    }

    /// Initialize this workspace into `'static` storage.
    pub fn init_static(
        storage: &'static StaticCell<Self>,
    ) -> &'static mut PixelBuffer<PIXEL_COUNT> {
        storage.init_with(Self::new)
    }

    /// Borrow a `width`×`height` view out of the workspace (must fit the capacity).
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

/// Draw buffer that a [`CydEsp`](super::CydEsp) can own and use to create
/// [`RegionView`]s. It works with any [`PixelBuffer<PIXEL_COUNT>`] size, while
/// the app chooses the capacity through `PIXEL_COUNT` on its
/// [`CydStaticEsp`](super::CydStaticEsp).
// TODO Consider replacing this trait-object capacity erasure with a concrete
// `PixelBufferView` over `&mut [u16]`, produced by `PixelBuffer::view`. Region
// borrowing can then live on the view without dynamic dispatch.
pub(crate) trait DynPixelBuffer: 'static {
    /// Borrow a `width`×`height` view out of the buffer (must fit the capacity).
    fn view_mut(&mut self, width: usize, height: usize) -> RegionView<'_>;
}

impl<const PIXEL_COUNT: usize> DynPixelBuffer for PixelBuffer<PIXEL_COUNT> {
    fn view_mut(&mut self, width: usize, height: usize) -> RegionView<'_> {
        PixelBuffer::view_mut(self, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::{DynPixelBuffer, PixelBuffer};
    use static_cell::StaticCell;

    #[test]
    #[should_panic(expected = "view must fit in workspace")]
    fn cyd_static_esp_frame_capacity_panics_when_region_is_too_large() {
        let mut pixel_buffer = PixelBuffer::<3>::new();
        pixel_buffer.view_mut(2, 2);
    }

    #[test]
    fn host_test_exercises_esp_pixel_buffer_path() {
        static STORAGE: StaticCell<PixelBuffer<1>> = StaticCell::new();
        let pixel_buffer = PixelBuffer::init_static(&STORAGE);
        let buffer: &mut dyn DynPixelBuffer = pixel_buffer;
        let mut view = buffer.view_mut(1, 1);
        assert_eq!(view.width(), 1);
        assert_eq!(view.height(), 1);
        view.raw_pixels_mut()[0] = 0x1234;
        assert_eq!(view.raw_pixels()[0], 0x1234);
    }
}

impl RegionView<'_> {
    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    /// Fill every pixel with `color`.
    pub fn fill(&mut self, color: Rgb565) {
        self.pixels.fill(color.into_storage());
    }

    /// Borrow the raw RGB565 pixels, row-major.
    pub fn raw_pixels_mut(&mut self) -> &mut [u16] {
        self.pixels
    }
    pub(crate) fn raw_pixels(&self) -> &[u16] {
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
