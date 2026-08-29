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

/// A read-only view of all or part of a statically stored RGB565 image.
///
/// A view borrows the original pixels without copying or allocating. Create a
/// full-image view with [`Image565Fixed::view`](super::tga::Image565Fixed::view),
/// or select a rectangular portion with
/// [`Image565Fixed::view_rect`](super::tga::Image565Fixed::view_rect).
///
/// Coordinates passed to [`Image565View::pixel_at`] are local to the view, so
/// `(0, 0)` addresses the crop's top-left pixel rather than the original
/// image's top-left pixel.
///
/// Views contain only color pixels and draw opaquely. For color-key
/// transparency, draw an [`Image565Fixed`](super::Image565Fixed) with a
/// [`MaskFixed`](super::MaskFixed), as shown in the
/// [`MaskFixed` example](super::MaskFixed).
///
/// # Example
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "image565_view",
        "docs/assets/image565_view.png"
    )
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::cyd::{
    Cyd, CydDisplay,
    display::{CydFrame, DrawItem, Image565Fixed, tga},
};
use embedded_graphics::{
    prelude::{Point, Size},
    primitives::Rectangle,
};

const IMAGE: Image565Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/assets/cyd_fill_contiguous.tga"
))
.to_565();

async fn draw<C: Cyd>(cyd: &mut C) -> Result<(), C::Error> {
    let full_view = IMAGE.view();
    let source = Rectangle::new(Point::new(2, 35), Size::new(41, 36));
    let cropped_view = IMAGE.view_rect(source);

    assert_eq!(cropped_view.size(), source.size);
    assert_eq!(
        cropped_view.pixel_at(Point::zero()),
        full_view.pixel_at(source.top_left),
    );
    assert_eq!(
        cropped_view.rgb565_iter().count(),
        source.size.width as usize * source.size.height as usize,
    );

    let display = cyd.display();
    let mut frame = display.full_frame_mut();
    DrawItem::Bitmap {
        view: full_view,
        top_left: Point::new(80, 84),
    }
    .draw(&mut frame);
    DrawItem::Bitmap {
        view: cropped_view,
        top_left: Point::new(200, 102),
    }
    .draw(&mut frame);
    frame.flush().await
}

# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{
#     mono_font::ascii::FONT_9X15_BOLD,
#     pixelcolor::Rgb888,
#     prelude::RgbColor,
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
#     "image565_view.png",
# );
# assert!(golden_result.is_ok(), "{golden_result:?}");
# Ok::<(), device_envoy_core::memory::Error>(())
```

The complete image is shown on the left and its cropped view on the right:

![A complete RGB565 image beside a cropped view of the same image][image565_view]
"#
)]
#[derive(Clone, Copy, Debug)]
pub struct Image565View {
    pixels: &'static [u16],
    stride: u32,
    source: Rectangle,
}

impl Image565View {
    /// Creates a full-image view from a row-major RGB565 pixel slice.
    ///
    /// Use this when pixels are already available as packed RGB565 values. For
    /// a compile-time TGA image, prefer [`Image565Fixed::view`](super::tga::Image565Fixed::view),
    /// as shown in the [`Image565View` example](Image565View).
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

    /// Returns this view's dimensions.
    ///
    /// For a cropped view, these are the crop dimensions rather than the full
    /// image dimensions. See the [`Image565View` example](Image565View).
    #[must_use]
    pub const fn size(&self) -> Size {
        self.source.size
    }

    /// Returns the pixel at a view-local coordinate.
    ///
    /// `(0, 0)` is the top-left of this view, not necessarily the top-left of
    /// the underlying image. See the [`Image565View` example](Image565View).
    ///
    /// Panics if `point` is outside the view.
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
    /// See the [`Image565View` example](Image565View).
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

/// A 2D drawing command that can be rendered onto a [`PixelTarget`].
///
/// `DrawItem` is a compact, [`Copy`] representation for a heterogeneous scene.
/// Its floating-point geometry is convenient for calculated or projected
/// coordinates, but projection is not required. The same items can be passed to
/// [`CydDisplay::draw_items`](crate::cyd::CydDisplay::draw_items), which
/// composites and streams them without a pixel frame buffer, or rendered
/// directly with [`DrawItem::draw`] when a frame is available.
///
/// For ordinary imperative drawing into a
/// [`CydFrame`](crate::cyd::display::CydFrame), embedded-graphics primitives are
/// also appropriate, particularly when their integer-coordinate geometry and
/// styling API fit the scene. `DrawItem::draw` uses embedded-graphics internally
/// for strokes and circles as an implementation detail; `DrawItem` does not
/// replace the broader embedded-graphics API.
///
/// Coordinates and sizes are measured in display pixels. Colors are specified
/// as [`Rgb888`], and the target converts them to its native pixel format when
/// needed.
///
/// # Example
///
/// This example loads a TGA bitmap at compile time and draws one of each item
/// variant onto a full-screen frame.
#[cfg_attr(
    feature = "doc-images",
    doc = ::embed_doc_image::embed_image!(
        "draw_item_bitmap",
        "docs/assets/draw_item_bitmap.png"
    )
)]
#[cfg_attr(
    feature = "host",
    doc = r#"

```rust
use device_envoy_core::cyd::{
    Cyd, CydDisplay,
    display::{CydFrame, DrawItem, Image565Fixed, tga},
};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};

const BITMAP: Image565Fixed<45, 73, { 45 * 73 }> = tga!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/assets/cyd_fill_contiguous.tga"
))
.to_565();

async fn draw<C: Cyd>(cyd: &mut C) -> Result<(), C::Error> {
    let display = cyd.display();
    let mut frame = display.full_frame_mut();
    let draw_items = [
        DrawItem::Bitmap {
            view: BITMAP.view(),
            top_left: Point::new(35, 84),
        },
        DrawItem::Circle {
            center: (125.0, 120.0),
            pixel_radius: 32.0,
            color: Rgb888::CYAN,
        },
        DrawItem::Ellipse {
            center: (215.0, 120.0),
            axis_a: (38.0, 0.0),
            axis_b: (12.0, 24.0),
            color: Rgb888::GREEN,
        },
        DrawItem::Stroke {
            start: (280.0, 80.0),
            end: (300.0, 160.0),
            color: Rgb888::YELLOW,
            pixel_width: 8.0,
        },
    ];
    for draw_item in draw_items {
        draw_item.draw(&mut frame);
    }
    frame.flush().await
}

# use device_envoy_core::memory::{CydMemory, assert_framebuffer_matches_expected_png};
# use embedded_graphics::{mono_font::ascii::FONT_9X15_BOLD, prelude::Size};
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
#     "draw_item_bitmap.png",
# );
# assert!(golden_result.is_ok(), "{golden_result:?}");
# Ok::<(), device_envoy_core::memory::Error>(())
```

![Examples of all four DrawItem variants][draw_item_bitmap]
"#
)]
#[derive(Clone, Copy, Debug)]
pub enum DrawItem {
    /// A line stroke from `start` to `end` with the given pixel width.
    Stroke {
        /// Start point in display coordinates.
        start: (f32, f32),
        /// End point in display coordinates.
        end: (f32, f32),
        /// Stroke color.
        color: Rgb888,
        /// Stroke width in pixels.
        pixel_width: f32,
    },
    /// A filled ellipse. It can also represent a projected disk.
    ///
    /// The ellipse is the locus of `center + s·axis_a + t·axis_b` with `s²+t² ≤ 1`.
    Ellipse {
        /// Center in display coordinates.
        center: (f32, f32),
        /// First radius vector, measured in pixels.
        axis_a: (f32, f32),
        /// Second radius vector, measured in pixels.
        axis_b: (f32, f32),
        /// Fill color.
        color: Rgb888,
    },
    /// A filled circle. It can also represent a projected sphere.
    Circle {
        /// Center in display coordinates.
        center: (f32, f32),
        /// Radius in pixels.
        pixel_radius: f32,
        /// Fill color.
        color: Rgb888,
    },
    /// A statically stored RGB565 bitmap placed at a display position.
    Bitmap {
        /// Bitmap pixels and dimensions.
        view: Image565View,
        /// Top-left corner in display coordinates.
        top_left: Point,
    },
}

impl DrawItem {
    /// Draw this item onto a [`PixelTarget`].
    ///
    /// Strokes use the embedded-graphics
    /// [`Line`](https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/struct.Line.html)
    /// primitive and circles use
    /// [`Circle`](https://docs.rs/embedded-graphics/latest/embedded_graphics/primitives/struct.Circle.html);
    /// the general ellipse is rasterized with
    /// [`fill_ellipse_pixels`].
    ///
    /// See the [`DrawItem` example](DrawItem).
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
