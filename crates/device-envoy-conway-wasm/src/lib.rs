use device_envoy_examples_core::conway_app::{ConwayApp, ConwayInput, ConwayStatus, HEIGHT, WIDTH};
use wasm_bindgen::prelude::*;

const DEFAULT_MAX_DIMENSION: u32 = 640;

/// Browser adapter for the shared Conway application.
#[wasm_bindgen]
pub struct ConwayWeb {
    app: ConwayApp,
}

#[wasm_bindgen]
impl ConwayWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            app: ConwayApp::new(),
        }
    }

    /// Forward one browser control key to the shared application.
    pub fn press_key(&mut self, key: &str) -> String {
        let Some(input) = input_for_key(key) else {
            return status_text(ConwayStatus::Unknown);
        };
        status_text(self.app.input(input))
    }

    /// Advance the shared application by one browser tick.
    pub fn tick(&mut self) -> String {
        status_text(self.app.tick())
    }

    /// Render the shared application as a PNG.
    pub fn render_png(&mut self) -> Result<Vec<u8>, JsValue> {
        self.render_png_with_max_dimension(DEFAULT_MAX_DIMENSION)
    }

    /// Render the shared application as a PNG with a maximum dimension.
    pub fn render_png_with_max_dimension(
        &mut self,
        max_dimension: u32,
    ) -> Result<Vec<u8>, JsValue> {
        self.app
            .frame()
            .to_png_bytes(max_dimension)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    /// Return the shared simulation's current animation interval.
    pub fn tick_interval_ms(&self) -> u32 {
        self.app.tick_interval_ms()
    }
}

impl Default for ConwayWeb {
    fn default() -> Self {
        Self::new()
    }
}

fn input_for_key(key: &str) -> Option<ConwayInput> {
    Some(match key {
        "prev" => ConwayInput::Previous,
        "next" => ConwayInput::Next,
        "play_pause" => ConwayInput::PlayPause,
        "cancel" => ConwayInput::Cancel,
        "speed_up" => ConwayInput::SpeedUp,
        "speed_down" => ConwayInput::SpeedDown,
        "mode" => ConwayInput::Mode,
        "power" => ConwayInput::Power,
        "repeat" => ConwayInput::Repeat,
        "usd" => ConwayInput::UndoSymmetry,
        "0" => ConwayInput::Pattern(0),
        "1" => ConwayInput::Pattern(1),
        "2" => ConwayInput::Pattern(2),
        "3" => ConwayInput::Pattern(3),
        "4" => ConwayInput::Pattern(4),
        "5" => ConwayInput::Pattern(5),
        "6" => ConwayInput::Pattern(6),
        "7" => ConwayInput::Pattern(7),
        "8" => ConwayInput::Pattern(8),
        "9" => ConwayInput::Pattern(9),
        _ => return None,
    })
}

fn status_text(status: ConwayStatus) -> String {
    match status {
        ConwayStatus::Ok => "ok",
        ConwayStatus::Searching => "searching",
        ConwayStatus::Found => "found",
        ConwayStatus::NotFound => "not_found",
        ConwayStatus::Cancelled => "cancelled",
        ConwayStatus::Paused => "paused",
        ConwayStatus::Unknown => "unknown",
    }
    .into()
}

const _: (usize, usize) = (WIDTH, HEIGHT);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_controls_forward_all_shared_display_inputs() {
        let mut conway = ConwayWeb::new();
        assert_eq!(conway.press_key("mode"), "ok");
        assert_eq!(conway.press_key("power"), "ok");
        assert_eq!(conway.press_key("not-a-control"), "unknown");
    }
}
