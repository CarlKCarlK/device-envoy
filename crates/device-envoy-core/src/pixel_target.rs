//! Generic RGB pixel-buffer helpers shared by display-style device modules.

use core::convert::Infallible;

use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{Dimensions, Point},
    pixelcolor::{Rgb565, Rgb888},
    prelude::RgbColor,
    primitives::Rectangle,
};

/// Rasterize an ellipse pixel-by-pixel via a callback.
///
/// The ellipse is the locus of `center + s·axis_a + t·axis_b` where `s²+t² ≤ 1`.
/// All values are in pixel space. Skips degenerate (edge-on) ellipses silently.
pub fn fill_ellipse_pixels(
    center: (f32, f32),
    axis_a: (f32, f32),
    axis_b: (f32, f32),
    mut put_pixel: impl FnMut(i32, i32),
) {
    let (axis_ax, axis_ay) = axis_a;
    let (axis_bx, axis_by) = axis_b;
    let determinant = axis_ax * axis_by - axis_ay * axis_bx;
    if determinant.abs() < 0.5 {
        return;
    }
    let inverse_determinant = 1.0 / determinant;
    let bound_x = (axis_ax.abs() + axis_bx.abs()) as i32 + 1;
    let bound_y = (axis_ay.abs() + axis_by.abs()) as i32 + 1;
    let center_x = center.0 as i32;
    let center_y = center.1 as i32;
    for local_y in -bound_y..=bound_y {
        for local_x in -bound_x..=bound_x {
            let delta_x = local_x as f32;
            let delta_y = local_y as f32;
            let s = (axis_by * delta_x - axis_bx * delta_y) * inverse_determinant;
            let t = (axis_ax * delta_y - axis_ay * delta_x) * inverse_determinant;
            if s * s + t * t <= 1.0 {
                put_pixel(center_x + local_x, center_y + local_y);
            }
        }
    }
}

/// A pixel sink that accepts individual RGB565 pixels at logical coordinates.
///
/// Every [`CydFrame`](crate::cyd::display::CydFrame) implements this trait, so
/// [`DrawItem`](crate::cyd::display::DrawItem) values can draw directly onto any
/// CYD frame. See the canonical [`DrawItem`](crate::cyd::display::DrawItem)
/// example for construction, rendering, and framebuffer verification.
pub trait PixelTarget: Dimensions {
    /// Set a pixel, ignoring points outside the target's bounds.
    ///
    /// The [`DrawItem`](crate::cyd::display::DrawItem) example demonstrates
    /// drawing through this operation.
    fn set_pixel(&mut self, point: Point, color: Rgb565);
}

/// Expands a packed RGB565 value (`RRRRR_GGGGGG_BBBBB`) to [`Rgb888`].
pub const fn rgb888_from_rgb565(rgb565: u16) -> Rgb888 {
    let red5 = ((rgb565 >> 11) & 0x1f) as u8;
    let green6 = ((rgb565 >> 5) & 0x3f) as u8;
    let blue5 = (rgb565 & 0x1f) as u8;

    let red = (red5 << 3) | (red5 >> 2);
    let green = (green6 << 2) | (green6 >> 4);
    let blue = (blue5 << 3) | (blue5 >> 2);

    Rgb888::new(red, green, blue)
}

/// Converts 8-bit RGB components to [`Rgb565`] by keeping each channel's high bits.
pub const fn rgb565_from_rgb888_components(red: u8, green: u8, blue: u8) -> Rgb565 {
    Rgb565::new(red >> 3, green >> 2, blue >> 3)
}

/// Converts [`Rgb888`] to [`Rgb565`].
pub fn rgb565_from_rgb888(color: Rgb888) -> Rgb565 {
    rgb565_from_rgb888_components(color.r(), color.g(), color.b())
}

/// Bridges a [`PixelTarget`] to the embedded-graphics [`DrawTarget`] interface.
pub struct PixelTargetAdapter<'a, T: PixelTarget>(pub &'a mut T);

impl<T: PixelTarget> DrawTarget for PixelTargetAdapter<'_, T> {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb888>>,
    {
        for Pixel(point, color) in pixels {
            self.0.set_pixel(point, Rgb565::from(color));
        }
        Ok(())
    }
}

impl<T: PixelTarget> Dimensions for PixelTargetAdapter<'_, T> {
    fn bounding_box(&self) -> Rectangle {
        self.0.bounding_box()
    }
}
