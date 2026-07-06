//! Display-only data, asset, and drawing plumbing for the CYD's `cyd` device
//! abstraction.
//!
//! The primary type is [`DrawItem`]; see [`CydDisplay::draw_items`](super::CydDisplay::draw_items)
//! for the canonical draw loop that consumes them.

mod contiguous_pixels;
mod draw_item;
mod orientation;
mod tga;

pub(crate) use contiguous_pixels::ContiguousPixels;
pub use draw_item::{DrawItem, Image565View};
pub use orientation::Orientation;
pub use tga::{Image565Fixed, Image565Mask};

pub use crate::{
    __cyd_tga565 as tga565, __cyd_tga565_magenta_mask as tga565_magenta_mask,
    __cyd_tga565_mask as tga565_mask, __cyd_tga565_white_mask as tga565_white_mask,
};

/// A borrowed or owned rectangular RGB565 pixel buffer.
pub trait RectanglePixels {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn raw_pixels(&self) -> &[u16];
}
