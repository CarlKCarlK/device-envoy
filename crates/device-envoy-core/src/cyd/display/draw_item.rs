use crate::pixel_target::{
    PixelTarget, PixelTargetAdapter, fill_ellipse_pixels, pixel_put, pixel_put_565,
};
use embedded_graphics::{
    Drawable,
    pixelcolor::{Rgb565, Rgb888, raw::RawU16},
    prelude::{IntoStorage, Point, Size},
    primitives::Rectangle,
    primitives::{Circle, Line, Primitive, PrimitiveStyle},
};

/// A view into a statically-stored RGB565 bitmap, optionally cropped to a
/// sub-rectangle.
///
/// For a full-image view use [`Image565Fixed::view`](super::tga::Image565Fixed::view);
/// for a cropped view use
/// [`Image565Fixed::view_rect`](super::tga::Image565Fixed::view_rect). `stride`
/// is the full image width (row step in pixels); `source` is the crop
/// rectangle in image coordinates.
///
/// ```rust,no_run
/// use device_envoy_core::cyd::display::Image565View;
/// use embedded_graphics::prelude::{Point, RgbColor, Size};
///
/// static PIXELS: [u16; 4] = [0, 0xffff, 0, 0xffff];
/// let image = Image565View::new(&PIXELS, Size::new(2, 2));
/// let _size = image.size();
/// let _pixel = image.pixel_at(Point::new(0, 0));
/// let _pixels: heapless::Vec<_, 4> = image.rgb565_iter().collect();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Image565View {
    pixels: &'static [u16],
    stride: u32,
    source: Rectangle,
}

impl Image565View {
    /// See the [canonical `Image565View` example](Image565View).
    /// Full-image view from a raw pixel slice.
    ///
    /// Panics if `pixels.len() != size.width * size.height`.
    #[must_use]
    pub const fn new(pixels: &'static [u16], size: Size) -> Self {
        assert!(
            pixels.len() == size.width as usize * size.height as usize,
            "Image565View pixels must match width * height"
        );
        Self {
            pixels,
            stride: size.width,
            source: Rectangle::new(Point::zero(), size),
        }
    }

    /// Cropped view — `source` is in image coordinates, `stride` is the full
    /// image row width. Prefer [`Image565Fixed::view_rect`] at call sites.
    #[must_use]
    pub(crate) const fn new_cropped(
        pixels: &'static [u16],
        stride: u32,
        source: Rectangle,
    ) -> Self {
        Self {
            pixels,
            stride,
            source,
        }
    }

    /// See the [canonical `Image565View` example](Image565View).
    #[must_use]
    pub const fn size(&self) -> Size {
        self.source.size
    }

    /// Returns the pixel at `point`, where `point` is in view-local coordinates
    /// (i.e. `(0, 0)` is the top-left of this view, not of the underlying image).
    /// See the [canonical `Image565View` example](Image565View).
    #[must_use]
    pub fn pixel_at(&self, point: Point) -> Rgb565 {
        assert!(
            point.x >= 0 && point.y >= 0,
            "Image565View pixel coordinate must be non-negative"
        );
        let vx = point.x as usize;
        let vy = point.y as usize;
        assert!(
            vx < self.source.size.width as usize && vy < self.source.size.height as usize,
            "Image565View pixel coordinate must be inside the view"
        );
        let source_x = self.source.top_left.x as usize + vx;
        let source_y = self.source.top_left.y as usize + vy;
        let index = source_y * self.stride as usize + source_x;
        Rgb565::from(RawU16::new(self.pixels[index]))
    }

    /// Iterate over the view's pixels in row-major order as `Rgb565` values.
    ///
    /// Cropped views skip the pixels outside the view while preserving the
    /// view's local row order.
    ///
    /// See the [canonical `Image565View` example](Image565View).
    pub fn rgb565_iter(&self) -> impl Iterator<Item = Rgb565> + '_ {
        Image565ViewPixels {
            view: *self,
            index: 0,
        }
    }
}

struct Image565ViewPixels {
    view: Image565View,
    index: usize,
}

impl Iterator for Image565ViewPixels {
    type Item = Rgb565;

    fn next(&mut self) -> Option<Self::Item> {
        let width = self.view.source.size.width as usize;
        let height = self.view.source.size.height as usize;
        if self.index >= width * height {
            return None;
        }

        let view_x = self.index % width;
        let view_y = self.index / width;
        let source_x = self.view.source.top_left.x as usize + view_x;
        let source_y = self.view.source.top_left.y as usize + view_y;
        let source_index = source_y * self.view.stride as usize + source_x;
        self.index += 1;
        Some(Rgb565::from(RawU16::new(self.view.pixels[source_index])))
    }
}

/// A pixel-space 2D draw item, ready to draw onto a [`PixelTarget`].
///
/// Construct one directly when you already have pixel-space geometry, or via
/// linkage-blaze's CYD 3D adapters when projecting a 3D scene. All coordinates
/// and sizes are in pixels. The `color` stays [`Rgb888`]; the target performs
/// any conversion (for example to `Rgb565`) at its pixel boundary.
///
/// ```rust,no_run
/// use device_envoy_core::{cyd::display::{DrawItem, Image565View}, pixel_target::PixelTarget};
/// use embedded_graphics::prelude::{Point, RgbColor, Size};
///
/// static PIXELS: [u16; 1] = [0xffff];
/// fn draw<T: PixelTarget>(target: &mut T) {
///     DrawItem::Circle { center: (4.0, 4.0), pixel_radius: 2.0, color: embedded_graphics::pixelcolor::Rgb888::WHITE }.draw(target);
///     DrawItem::Bitmap { view: Image565View::new(&PIXELS, Size::new(1, 1)), top_left: Point::zero() }.draw(target);
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub enum DrawItem {
    /// A line stroke from `start` to `end` with the given pixel width.
    Stroke {
        start: (f32, f32),
        end: (f32, f32),
        color: Rgb888,
        pixel_width: f32,
    },
    /// A filled, possibly foreshortened, ellipse (a projected disk).
    ///
    /// The ellipse is the locus of `center + s·axis_a + t·axis_b` with `s²+t² ≤ 1`.
    Ellipse {
        center: (f32, f32),
        axis_a: (f32, f32),
        axis_b: (f32, f32),
        color: Rgb888,
    },
    /// A filled circle (a projected sphere).
    Circle {
        center: (f32, f32),
        pixel_radius: f32,
        color: Rgb888,
    },
    /// A statically-stored RGB565 bitmap view placed at a screen position.
    Bitmap { view: Image565View, top_left: Point },
}

impl DrawItem {
    /// Draw this item onto a [`PixelTarget`].
    ///
    /// Strokes use the embedded-graphics [`Line`] primitive and circles use
    /// [`Circle`]; the general projected ellipse is rasterized with
    /// [`fill_ellipse_pixels`].
    ///
    /// See the [canonical `DrawItem` example](DrawItem).
    pub fn draw<T: PixelTarget>(&self, target: &mut T) {
        match *self {
            DrawItem::Stroke {
                start,
                end,
                color,
                pixel_width,
            } => {
                let width = ((pixel_width + 0.5) as u32).max(1);
                Line::new(
                    embedded_graphics::prelude::Point::new(start.0 as i32, start.1 as i32),
                    embedded_graphics::prelude::Point::new(end.0 as i32, end.1 as i32),
                )
                .into_styled(PrimitiveStyle::with_stroke(color, width))
                .draw(&mut PixelTargetAdapter(target))
                .expect("drawing onto a PixelTargetAdapter is Infallible");
            }
            DrawItem::Ellipse {
                center,
                axis_a,
                axis_b,
                color,
            } => {
                fill_ellipse_pixels(center, axis_a, axis_b, |position_x, position_y| {
                    pixel_put(target, position_x, position_y, color);
                });
            }
            DrawItem::Circle {
                center,
                pixel_radius,
                color,
            } => {
                let diameter = (((pixel_radius * 2.0) + 0.5) as u32).max(1);
                Circle::with_center(
                    embedded_graphics::prelude::Point::new(center.0 as i32, center.1 as i32),
                    diameter,
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(&mut PixelTargetAdapter(target))
                .expect("drawing onto a PixelTargetAdapter is Infallible");
            }
            DrawItem::Bitmap { view, top_left } => {
                let size = view.size();
                for dy in 0..size.height as i32 {
                    for dx in 0..size.width as i32 {
                        let view_point = Point::new(dx, dy);
                        let target_point = top_left + view_point;
                        pixel_put_565(
                            target,
                            target_point.x,
                            target_point.y,
                            view.pixel_at(view_point).into_storage(),
                        );
                    }
                }
            }
        }
    }
}
