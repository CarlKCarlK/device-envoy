//! A device abstraction for rectangular NeoPixel-style (WS2812) LED panel displays.
//! See [`Led2dEsp`] for the runtime adapter and [`layout::LedLayout`] for compile-time
//! panel wiring and geometry.
//!
pub mod layout {
    pub use device_envoy_core::led2d::layout::*;
}

pub use device_envoy_core::led2d::Led2d;
pub use device_envoy_core::led2d::{
    bit_matrix3x4_font, render_text_to_frame, Frame2d, Led2dFont, Led2dStripAdapter,
    Led2dStripBacked, LedLayout, Point, Size,
};
pub mod led2d_generated;

pub type Led2dEsp<'a, const N: usize, S> = Led2dStripAdapter<'a, N, S>;

/// Generate a 2D panel device type backed by `led_strip!`.
///
/// Required fields:
/// - `pin`
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            $pin,
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
            pin: $pin:ident,
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
            pin: $pin:ident,
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
            $pin,
            $len,
            $max_current,
            [$($gamma)?],
            [$($max_frames)?],
            [$led_layout],
            [$font],
        }
    };
}
