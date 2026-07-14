//! Shared browser-facing construction and input control for a simulated CYD.

use embedded_graphics::{
    mono_font::{MonoFont, ascii::FONT_6X10},
    pixelcolor::Rgb888,
};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::{ButtonWasm, ButtonWasmSource, CydTouchWasmSource, CydWasm};
use crate::cyd::display::Orientation;

const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white

/// The reusable resources and input protocol for one browser CYD instance.
pub struct CydSimulatorWasm {
    cyd: CydWasm,
    button_source: ButtonWasmSource,
    control: CydSimulatorControlWasm,
}

/// Browser input and lifecycle control shared by an application launcher.
#[wasm_bindgen]
#[derive(Clone)]
pub struct CydSimulatorControlWasm {
    touch_source: CydTouchWasmSource,
    button_source: ButtonWasmSource,
    orientation: Orientation,
}

impl CydSimulatorWasm {
    /// Construct a simulated CYD using the standard CYD browser palette.
    pub fn new(canvas: HtmlCanvasElement, orientation: Orientation) -> Result<Self, JsValue> {
        Self::new_with_style(canvas, orientation, BACKGROUND, FOREGROUND, &FONT_6X10)
    }

    /// Construct a simulated CYD with an application-specific display style.
    pub fn new_with_style(
        canvas: HtmlCanvasElement,
        orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Result<Self, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2D canvas context unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        canvas.set_width(orientation.width());
        canvas.set_height(orientation.height());

        let touch_source = CydTouchWasmSource::new();
        let button_source = ButtonWasmSource::new();
        let cyd = CydWasm::new(
            context,
            orientation,
            background,
            foreground,
            font,
            touch_source.clone(),
        );
        let control = CydSimulatorControlWasm {
            touch_source,
            button_source: button_source.clone(),
            orientation,
        };
        Ok(Self {
            cyd,
            button_source,
            control,
        })
    }

    /// Split the simulator into application device resources and browser control.
    pub fn into_parts(self) -> (CydWasm, ButtonWasm, CydSimulatorControlWasm) {
        let Self {
            cyd,
            button_source,
            control,
        } = self;
        (cyd, button_source.button(), control)
    }
}

impl CydSimulatorControlWasm {
    /// Return the display orientation used by this simulator instance.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.orientation
    }
}

#[wasm_bindgen]
impl CydSimulatorControlWasm {
    /// Forward a browser pointer-down position in logical canvas coordinates.
    #[wasm_bindgen(js_name = touch_down)]
    pub fn touch_down(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation, x, y);
        self.touch_source.touch_down(point.0, point.1);
    }

    /// Forward a browser pointer-move position in logical canvas coordinates.
    #[wasm_bindgen(js_name = touch_move)]
    pub fn touch_move(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation, x, y);
        self.touch_source.touch_move(point.0, point.1);
    }

    /// Forward a browser pointer-up or pointer-cancel event.
    #[wasm_bindgen(js_name = touch_up)]
    pub fn touch_up(&self) {
        self.touch_source.touch_up();
    }

    /// Forward a physical BOOT-button press.
    #[wasm_bindgen(js_name = boot_down)]
    pub fn boot_down(&self) {
        self.button_source.press();
    }

    /// Forward a physical BOOT-button release.
    #[wasm_bindgen(js_name = boot_up)]
    pub fn boot_up(&self) {
        self.button_source.release();
    }

    /// Clear transient browser input after a simulated reset.
    pub fn reset_transient_state(&self) {
        self.touch_source.touch_up();
        self.button_source.release();
    }
}

fn map_to_landscape(orientation: Orientation, x: f32, y: f32) -> (f32, f32) {
    match orientation {
        Orientation::Landscape => (x, y),
        Orientation::Portrait => (319.0 - y, x),
        Orientation::LandscapeInverted => (319.0 - x, 239.0 - y),
        Orientation::PortraitInverted => (y, 239.0 - x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_mapping_round_trips() {
        for orientation in [
            Orientation::Landscape,
            Orientation::Portrait,
            Orientation::LandscapeInverted,
            Orientation::PortraitInverted,
        ] {
            let landscape_point = map_to_landscape(orientation, 37.0, 83.0);
            let logical_point = match orientation {
                Orientation::Landscape => landscape_point,
                Orientation::Portrait => (landscape_point.1, 319.0 - landscape_point.0),
                Orientation::LandscapeInverted => {
                    (319.0 - landscape_point.0, 239.0 - landscape_point.1)
                }
                Orientation::PortraitInverted => (239.0 - landscape_point.1, landscape_point.0),
            };
            assert_eq!(logical_point, (37.0, 83.0));
        }
    }
}
