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
/// - `font`
///
/// Optional fields:
/// - `max_current` (default [`crate::led_strip::CURRENT_DEFAULT`])
/// - `engine` (default `Engine::Rmt`)
/// - `gamma`
/// - `max_frames`
#[doc(hidden)]
#[macro_export]
macro_rules! led2d {
    (
        $name:ident {
            $($fields:tt)*
        }
    ) => {
        $crate::__led2d_collect_fields!{
            $name,
            pin = [],
            len = [],
            led_layout = [],
            max_current = [],
            font = [],
            engine = [],
            gamma = [],
            max_frames = [],
            fields = [$($fields)*],
        }
    };
}

#[cfg(target_os = "none")]
#[doc(inline)]
pub use led2d;

#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_collect_fields {
    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [$led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [$font:expr],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        $crate::__led2d_dispatch_engine!(
            $name,
            $pin,
            $len,
            $led_layout,
            $crate::__led_strip_max_current_or_default!([$($max_current)?]),
            $font,
            [$($engine)?],
            [$($gamma)?],
            [$($max_frames)?],
        );
    };
    (
        $name:ident,
        pin = [],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `pin` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `len` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `led_layout` field");
    };
    (
        $name:ident,
        pin = [$pin:ident],
        len = [$len:expr],
        led_layout = [$led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [],
    ) => {
        compile_error!("led2d! missing required `font` field");
    };
    (
        $name:ident,
        pin = [],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$pin],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$already_pin:ident],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [pin: $pin:ident $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `pin` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$len],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$already_len:expr],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [len: $len:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `len` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [led_layout: $led_layout:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$led_layout],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$already_led_layout:expr],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [led_layout: $led_layout:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `led_layout` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$max_current],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$already_max_current:expr],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [max_current: $max_current:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `max_current` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [font: $font:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$font],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$already_font:expr],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [font: $font:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `font` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $crate::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Spi $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Spi],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $crate::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: device_envoy_esp::led_strip::Engine::Rmt $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [Rmt],
            gamma = [$($gamma)?],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$already_engine:tt],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `engine` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [engine: $ignored:path $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! engine must be Engine::Rmt or Engine::Spi");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [],
        max_frames = [$($max_frames:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$gamma],
            max_frames = [$($max_frames)?],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$already_gamma:expr],
        max_frames = [$($max_frames:expr)?],
        fields = [gamma: $gamma:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `gamma` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        $crate::__led2d_collect_fields!(
            $name,
            pin = [$($pin)?],
            len = [$($len)?],
            led_layout = [$($led_layout)?],
            max_current = [$($max_current)?],
            font = [$($font)?],
            engine = [$($engine)?],
            gamma = [$($gamma)?],
            max_frames = [$max_frames],
            fields = [$($($rest)*)?],
        );
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$already_max_frames:expr],
        fields = [max_frames: $max_frames:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!("led2d! duplicate `max_frames` field");
    };
    (
        $name:ident,
        pin = [$($pin:ident)?],
        len = [$($len:expr)?],
        led_layout = [$($led_layout:expr)?],
        max_current = [$($max_current:expr)?],
        font = [$($font:expr)?],
        engine = [$($engine:tt)?],
        gamma = [$($gamma:expr)?],
        max_frames = [$($max_frames:expr)?],
        fields = [$field:ident : $value:expr $(, $($rest:tt)*)?],
    ) => {
        compile_error!(
            "led2d! unknown field; expected `pin`, `len`, `led_layout`, `font`, `max_current`, `engine`, `gamma`, or `max_frames`"
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __led2d_dispatch_engine {
    (
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [Spi],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
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
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [Rmt],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
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
        $name:ident,
        $pin:ident,
        $len:expr,
        $led_layout:expr,
        $max_current:expr,
        $font:expr,
        [],
        [$($gamma:expr)?],
        [$($max_frames:expr)?],
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
