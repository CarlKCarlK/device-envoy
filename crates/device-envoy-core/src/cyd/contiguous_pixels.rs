use embedded_graphics::{pixelcolor::Rgb565, prelude::Point, primitives::Rectangle};

use super::{DrawItem, Image565View};

#[derive(Clone, Copy, Debug)]
struct PreparedBounds {
    left: i32,
    top: i32,
    right_exclusive: i32,
    bottom_exclusive: i32,
}

impl PreparedBounds {
    fn contains(&self, point_x: i32, point_y: i32) -> bool {
        self.contains_x(point_x) && self.contains_y(point_y)
    }

    fn contains_x(&self, point_x: i32) -> bool {
        self.left <= point_x && point_x < self.right_exclusive
    }

    fn contains_y(&self, point_y: i32) -> bool {
        self.top <= point_y && point_y < self.bottom_exclusive
    }
}

#[derive(Clone, Copy, Debug)]
struct PreparedPixelSource {
    bounds: PreparedBounds,
    kind: PreparedPixelSourceKind,
}

#[derive(Clone, Copy, Debug)]
enum PreparedPixelSourceKind {
    Primitive(PreparedPrimitive),
    Bitmap { view: Image565View, top_left: Point },
}

#[derive(Clone, Copy, Debug)]
struct PreparedPrimitive {
    color: Rgb565,
    kind: PreparedKind,
}

#[derive(Clone, Copy, Debug)]
enum PreparedKind {
    Line(PreparedLine),
    Ellipse(PreparedEllipse),
}

#[derive(Clone, Copy, Debug)]
struct PreparedLine {
    start_x: i64,
    start_y: i64,
    end_x: i64,
    end_y: i64,
    segment_x: i64,
    segment_y: i64,
    segment_len_squared: i64,
    radius_squared: i64,
    radius_squared_times_len_squared: i64,
}

#[derive(Clone, Copy, Debug)]
struct PreparedEllipse {
    center_x: i32,
    center_y: i32,
    quadratic_xx: f32,
    quadratic_xy: f32,
    quadratic_yy: f32,
    outer_limit: f32,
}

impl PreparedPrimitive {
    // todo: review the color (Rgb888 -> Rgb565) and number (f32 -> i32/u16)
    // conversions threaded through here and `DrawItem`. Some may be
    // happening later (per primitive, per frame) than strictly needed — see if
    // any can move once, earlier, or be dropped entirely.
    fn from_projected(item: &DrawItem) -> Option<PreparedPixelSource> {
        match *item {
            DrawItem::Stroke {
                start,
                end,
                color,
                pixel_width,
            } => {
                let start = point_from_f32(start);
                let end = point_from_f32(end);
                let width = pixel_width_u16(pixel_width);
                if start == end {
                    return None;
                }
                let start_x = start.x;
                let start_y = start.y;
                let end_x = end.x;
                let end_y = end.y;
                let segment_x = i64::from(end_x - start_x);
                let segment_y = i64::from(end_y - start_y);
                let segment_len_squared = segment_x * segment_x + segment_y * segment_y;
                let radius = (i64::from(width) + 1) / 2;
                let radius_squared = radius * radius;

                let min_x = start_x.min(end_x) - radius as i32;
                let max_x = start_x.max(end_x) + radius as i32;
                let min_y = start_y.min(end_y) - radius as i32;
                let max_y = start_y.max(end_y) + radius as i32;

                Some(PreparedPixelSource {
                    bounds: PreparedBounds {
                        left: min_x,
                        top: min_y,
                        right_exclusive: max_x + 1,
                        bottom_exclusive: max_y + 1,
                    },
                    kind: PreparedPixelSourceKind::Primitive(PreparedPrimitive {
                        color: Rgb565::from(color),
                        kind: PreparedKind::Line(PreparedLine {
                            start_x: i64::from(start_x),
                            start_y: i64::from(start_y),
                            end_x: i64::from(end_x),
                            end_y: i64::from(end_y),
                            segment_x,
                            segment_y,
                            segment_len_squared,
                            radius_squared,
                            radius_squared_times_len_squared: radius_squared
                                .checked_mul(segment_len_squared)
                                .expect("prepared line radius and length product must fit in i64"),
                        }),
                    }),
                })
            }
            DrawItem::Ellipse {
                center,
                axis_a,
                axis_b,
                color,
            } => Self::prepare_ellipse(
                point_from_f32(center),
                axis_a,
                axis_b,
                ellipse_bound_radius(axis_a, axis_b),
                Rgb565::from(color),
            ),
            DrawItem::Circle {
                center,
                pixel_radius,
                color,
            } => Self::prepare_ellipse(
                point_from_f32(center),
                (pixel_radius, 0.0),
                (0.0, pixel_radius),
                pixel_radius,
                Rgb565::from(color),
            ),
            DrawItem::Bitmap { view, top_left } => {
                let bounds = Rectangle::new(top_left, view.size());
                if bounds.size.width == 0 || bounds.size.height == 0 {
                    return None;
                }
                Some(PreparedPixelSource {
                    bounds: bounds_from_rectangle(bounds),
                    kind: PreparedPixelSourceKind::Bitmap { view, top_left },
                })
            }
        }
    }

    fn prepare_ellipse(
        center: Point,
        axis_a: (f32, f32),
        axis_b: (f32, f32),
        radius: f32,
        color: Rgb565,
    ) -> Option<PreparedPixelSource> {
        let det = axis_a.0 * axis_b.1 - axis_a.1 * axis_b.0;
        let det_squared = det * det;

        if det_squared == 0.0 {
            return None;
        }

        let (ax, ay) = axis_a;
        let (bx, by) = axis_b;
        let quadratic_xx = by * by + ay * ay;
        let quadratic_xy = -2.0 * (by * bx + ay * ax);
        let quadratic_yy = bx * bx + ax * ax;

        let radius = ceil_nonnegative_f32(radius) + 1;
        let left = center.x - radius;
        let top = center.y - radius;
        let right_exclusive = center.x + radius;
        let bottom_exclusive = center.y + radius;

        Some(PreparedPixelSource {
            bounds: PreparedBounds {
                left,
                top,
                right_exclusive,
                bottom_exclusive,
            },
            kind: PreparedPixelSourceKind::Primitive(PreparedPrimitive {
                color,
                kind: PreparedKind::Ellipse(PreparedEllipse {
                    center_x: center.x,
                    center_y: center.y,
                    quadratic_xx,
                    quadratic_xy,
                    quadratic_yy,
                    outer_limit: det_squared,
                }),
            }),
        })
    }
}

/// A compiled row-major RGB565 view of projected draw items inside one rectangle.
pub struct ContiguousPixels<const PIXEL_SOURCE_COUNT: usize> {
    bounds: Rectangle,
    left: i32,
    top: i32,
    right_exclusive: i32,
    bottom_exclusive: i32,
    background: Rgb565,
    pixel_sources: heapless::Vec<PreparedPixelSource, PIXEL_SOURCE_COUNT>,
}

impl<const PIXEL_SOURCE_COUNT: usize> ContiguousPixels<PIXEL_SOURCE_COUNT> {
    /// Compile already-projected draw items for indexed pixel lookups.
    #[must_use]
    pub fn from_draw_items(
        bounds: Rectangle,
        background: Rgb565,
        draw_items: impl IntoIterator<Item = DrawItem>,
    ) -> Self {
        let mut pixel_sources = heapless::Vec::<PreparedPixelSource, PIXEL_SOURCE_COUNT>::new();
        for draw_item in draw_items {
            if let Some(prepared_pixel_source) = PreparedPrimitive::from_projected(&draw_item) {
                pixel_sources
                    .push(prepared_pixel_source)
                    .expect("projected draw items fit the prepared pixel source capacity");
            }
        }

        Self::new(bounds, background, pixel_sources)
    }

    fn new(
        bounds: Rectangle,
        background: Rgb565,
        pixel_sources: heapless::Vec<PreparedPixelSource, PIXEL_SOURCE_COUNT>,
    ) -> Self {
        let left = bounds.top_left.x;
        let top = bounds.top_left.y;
        let right_exclusive = left
            .checked_add(bounds.size.width as i32)
            .expect("compiled primitive bounds right edge must fit in i32");
        let bottom_exclusive = top
            .checked_add(bounds.size.height as i32)
            .expect("compiled primitive bounds bottom edge must fit in i32");

        Self {
            bounds,
            left,
            top,
            right_exclusive,
            bottom_exclusive,
            background,
            pixel_sources,
        }
    }

    #[must_use]
    pub fn bounds(&self) -> Rectangle {
        self.bounds
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left >= self.right_exclusive || self.top >= self.bottom_exclusive
    }

    #[must_use]
    pub fn pixel_at(&self, point_x: i32, point_y: i32) -> Rgb565 {
        for pixel_source in self.pixel_sources.iter().rev() {
            if let Some(color) = pixel_source.color_at(point_x, point_y) {
                return color;
            }
        }

        self.background
    }

    #[must_use]
    pub fn iter(&self) -> ContiguousPixelsIter<'_, PIXEL_SOURCE_COUNT> {
        ContiguousPixelsIter::new(self)
    }
}

impl PreparedPixelSource {
    fn color_at(&self, point_x: i32, point_y: i32) -> Option<Rgb565> {
        if !self.bounds.contains(point_x, point_y) {
            return None;
        }

        match self.kind {
            PreparedPixelSourceKind::Primitive(primitive) => primitive
                .covers_inside_bounds(point_x, point_y)
                .then_some(primitive.color),
            PreparedPixelSourceKind::Bitmap { view, top_left } => {
                Some(view.pixel_at(Point::new(point_x, point_y) - top_left))
            }
        }
    }
}

impl PreparedPrimitive {
    fn covers_inside_bounds(&self, point_x: i32, point_y: i32) -> bool {
        match self.kind {
            PreparedKind::Line(line) => line.covers(point_x, point_y),
            PreparedKind::Ellipse(ellipse) => ellipse.covers(point_x, point_y),
        }
    }
}

impl PreparedLine {
    fn covers(&self, point_x: i32, point_y: i32) -> bool {
        point_covered_by_prepared_segment_fast(
            point_x,
            point_y,
            self.start_x,
            self.start_y,
            self.end_x,
            self.end_y,
            self.segment_x,
            self.segment_y,
            self.segment_len_squared,
            self.radius_squared,
            self.radius_squared_times_len_squared,
        )
    }
}

impl PreparedEllipse {
    fn covers(&self, point_x: i32, point_y: i32) -> bool {
        point_covered_by_prepared_ellipse(
            point_x,
            point_y,
            self.center_x,
            self.center_y,
            self.quadratic_xx,
            self.quadratic_xy,
            self.quadratic_yy,
            self.outer_limit,
        )
    }
}

pub struct ContiguousPixelsIter<'a, const PIXEL_SOURCE_COUNT: usize> {
    pixels: &'a ContiguousPixels<PIXEL_SOURCE_COUNT>,
    x: i32,
    y: i32,
    active: heapless::Vec<&'a PreparedPixelSource, PIXEL_SOURCE_COUNT>,
}

impl<'a, const PIXEL_SOURCE_COUNT: usize> ContiguousPixelsIter<'a, PIXEL_SOURCE_COUNT> {
    fn new(pixels: &'a ContiguousPixels<PIXEL_SOURCE_COUNT>) -> Self {
        let mut contiguous_pixels_iter = Self {
            pixels,
            x: pixels.left,
            y: pixels.top,
            active: heapless::Vec::new(),
        };
        contiguous_pixels_iter.rebuild_active();
        contiguous_pixels_iter
    }

    fn rebuild_active(&mut self) {
        self.active.clear();
        for pixel_source in self.pixels.pixel_sources.iter().rev() {
            if pixel_source.bounds.contains_y(self.y) {
                self.active
                    .push(pixel_source)
                    .expect("active pixel source references fit active list capacity");
            }
        }
    }
}

impl<const PIXEL_SOURCE_COUNT: usize> Iterator for ContiguousPixelsIter<'_, PIXEL_SOURCE_COUNT> {
    type Item = Rgb565;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.pixels.right_exclusive || self.y >= self.pixels.bottom_exclusive {
            return None;
        }

        let mut color = self.pixels.background;
        for pixel_source in &self.active {
            if !pixel_source.bounds.contains_x(self.x) {
                continue;
            }
            if let Some(source_color) = pixel_source.color_at(self.x, self.y) {
                color = source_color;
                break;
            }
        }

        self.x += 1;
        if self.x >= self.pixels.right_exclusive {
            self.x = self.pixels.left;
            self.y += 1;
            if self.y < self.pixels.bottom_exclusive {
                self.rebuild_active();
            }
        }

        Some(color)
    }
}

fn point_covered_by_prepared_segment_fast(
    point_x: i32,
    point_y: i32,
    start_x: i64,
    start_y: i64,
    end_x: i64,
    end_y: i64,
    segment_x: i64,
    segment_y: i64,
    segment_len_squared: i64,
    radius_squared: i64,
    radius_squared_times_len_squared: i64,
) -> bool {
    let point_x = i64::from(point_x);
    let point_y = i64::from(point_y);
    let point_from_start_x = point_x - start_x;
    let point_from_start_y = point_y - start_y;

    let dot = point_from_start_x * segment_x + point_from_start_y * segment_y;
    if dot <= 0 {
        return point_from_start_x * point_from_start_x + point_from_start_y * point_from_start_y
            <= radius_squared;
    }

    if dot >= segment_len_squared {
        let distance_x = point_x - end_x;
        let distance_y = point_y - end_y;
        return distance_x * distance_x + distance_y * distance_y <= radius_squared;
    }

    let cross = point_from_start_x * segment_y - point_from_start_y * segment_x;
    cross * cross <= radius_squared_times_len_squared
}

fn point_covered_by_prepared_ellipse(
    point_x: i32,
    point_y: i32,
    center_x: i32,
    center_y: i32,
    quadratic_xx: f32,
    quadratic_xy: f32,
    quadratic_yy: f32,
    outer_limit: f32,
) -> bool {
    let dx = (point_x - center_x) as f32;
    let dy = (point_y - center_y) as f32;
    let distance_squared = quadratic_xx * dx * dx + quadratic_xy * dx * dy + quadratic_yy * dy * dy;

    distance_squared <= outer_limit
}

fn point_from_f32((x, y): (f32, f32)) -> Point {
    Point::new(x as i32, y as i32)
}

fn bounds_from_rectangle(rectangle: Rectangle) -> PreparedBounds {
    let left = rectangle.top_left.x;
    let top = rectangle.top_left.y;
    let right_exclusive = left
        .checked_add(rectangle.size.width as i32)
        .expect("rectangle right edge must fit in i32");
    let bottom_exclusive = top
        .checked_add(rectangle.size.height as i32)
        .expect("rectangle bottom edge must fit in i32");
    PreparedBounds {
        left,
        top,
        right_exclusive,
        bottom_exclusive,
    }
}

fn pixel_width_u16(width: f32) -> u16 {
    ((width + 0.5) as u16).max(1)
}

fn ellipse_bound_radius(axis_a: (f32, f32), axis_b: (f32, f32)) -> f32 {
    (abs_f32(axis_a.0) + abs_f32(axis_b.0)).max(abs_f32(axis_a.1) + abs_f32(axis_b.1))
}

fn abs_f32(value: f32) -> f32 {
    if value < 0.0 { -value } else { value }
}

fn ceil_nonnegative_f32(value: f32) -> i32 {
    assert!(
        value >= 0.0,
        "ceil_nonnegative_f32 input must be non-negative"
    );
    let truncated = value as i32;
    if value > truncated as f32 {
        truncated + 1
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use embedded_graphics::{
        pixelcolor::{Rgb565, Rgb888, RgbColor, WebColors, raw::RawU16},
        prelude::{IntoStorage, Point, Size},
        primitives::Rectangle,
    };

    use super::super::Image565View;
    use super::*;

    static BITMAP_PIXELS: [u16; 4] = [0x0000, 0xffff, 0xf800, 0x07e0];

    #[test]
    fn bitmap_item_samples_as_background_under_later_items() {
        let bitmap = Image565View::new(&BITMAP_PIXELS, Size::new(2, 2));
        let bitmap_item = DrawItem::Bitmap {
            view: bitmap,
            top_left: Point::zero(),
        };
        let circle = DrawItem::Circle {
            center: (0.0, 0.0),
            pixel_radius: 0.1,
            color: Rgb888::CSS_BLUE,
        };

        let contiguous_pixels = ContiguousPixels::<2>::from_draw_items(
            Rectangle::new(Point::zero(), Size::new(2, 2)),
            Rgb565::BLACK,
            [bitmap_item, circle],
        );

        assert_eq!(
            contiguous_pixels.pixel_at(1, 0).into_storage(),
            BITMAP_PIXELS[1]
        );
        assert_eq!(
            contiguous_pixels.pixel_at(0, 0).into_storage(),
            Rgb565::from(Rgb888::CSS_BLUE).into_storage()
        );
        assert_eq!(
            contiguous_pixels.pixel_at(0, 1).into_storage(),
            Rgb565::from(RawU16::new(BITMAP_PIXELS[2])).into_storage()
        );
    }
}
