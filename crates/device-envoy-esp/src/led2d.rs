//! A device abstraction for rectangular NeoPixel-style (WS2812) LED panel displays.
//! See [`Led2d`] for the runtime adapter and [`layout::LedLayout`] for compile-time
//! panel wiring and geometry.

pub mod layout {
    pub use device_envoy_core::led2d::layout::*;
}

pub use device_envoy_core::led2d::{
    bit_matrix3x4_font, render_text_to_frame, Frame2d, Led2dFont, LedLayout, Point, Size,
};
pub use device_envoy_core::led2d::Led2d as Led2dApi;

use core::borrow::Borrow;
use smart_leds::RGB8;

use crate::led_strip::{Frame1d as StripFrame, LedStrip};

pub struct Led2d<'a, const N: usize, const MAX_FRAMES: usize> {
    led_strip: &'a LedStrip<N, MAX_FRAMES>,
    mapping_by_xy: [u16; N],
    width: usize,
}

impl<'a, const N: usize, const MAX_FRAMES: usize> Led2d<'a, N, MAX_FRAMES> {
    #[must_use]
    pub fn new<const W: usize, const H: usize>(
        led_strip: &'a LedStrip<N, MAX_FRAMES>,
        led_layout: &LedLayout<N, W, H>,
    ) -> Self {
        assert_eq!(
            W.checked_mul(H).expect("width * height must fit in usize"),
            N,
            "width * height must equal N"
        );
        Self {
            led_strip,
            mapping_by_xy: led_layout.xy_to_index(),
            width: W,
        }
    }

    #[must_use]
    fn xy_to_index(&self, x_index: usize, y_index: usize) -> usize {
        self.mapping_by_xy[y_index * self.width + x_index] as usize
    }

    fn convert_frame<const W: usize, const H: usize>(
        &self,
        frame_2d: Frame2d<W, H>,
    ) -> StripFrame<N> {
        let mut frame_1d = [RGB8::new(0, 0, 0); N];
        for y_index in 0..H {
            for x_index in 0..W {
                let led_index = self.xy_to_index(x_index, y_index);
                frame_1d[led_index] = frame_2d[(x_index, y_index)];
            }
        }
        StripFrame::from(frame_1d)
    }

    pub fn write_frame<const W: usize, const H: usize>(&self, frame: Frame2d<W, H>) {
        let strip_frame = self.convert_frame(frame);
        self.led_strip.write_frame(strip_frame);
    }

    pub fn animate<const W: usize, const H: usize, I>(&self, frames: I)
    where
        I: IntoIterator,
        I::Item: Borrow<(Frame2d<W, H>, embassy_time::Duration)>,
    {
        self.led_strip.animate(frames.into_iter().map(|frame| {
            let (frame, duration) = *frame.borrow();
            (self.convert_frame(frame), duration)
        }));
    }
}

/// Generate a 2D panel device type backed by `led_strip!`.
///
/// Required fields:
/// - `len`
/// - `led_layout`
/// - `max_current`
/// - `font`
///
/// Optional fields:
/// - `engine` (default `Engine::Rmt`)
/// - `gamma`
/// - `max_frames`
#[macro_export]
macro_rules! led2d {
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: $crate::led_strip::Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: device_envoy_esp::led_strip::Engine::Spi
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::spi::__led_strip_spi_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: $crate::led_strip::Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: device_envoy_esp::led_strip::Engine::Rmt
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr,
            engine: $engine:path
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        compile_error!("led2d! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        $name:ident {
            len: $len:expr,
            led_layout: $led_layout:expr,
            max_current: $max_current:expr,
            font: $font:expr
            $(, gamma: $gamma:expr)?
            $(, max_frames: $max_frames:expr)?
            $(,)?
        }
    ) => {
        $crate::led_strip::__led_strip_inner!{
            $name,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
}
