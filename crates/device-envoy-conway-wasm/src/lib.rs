use device_envoy_conway_core::{Conway, ConwayCommand};
use wasm_bindgen::prelude::*;

const WIDTH: usize = 16;
const HEIGHT: usize = 16;
const DEFAULT_MAX_DIMENSION: u32 = 640;

#[wasm_bindgen]
pub struct ConwayWeb {
    conway: Conway<HEIGHT, WIDTH>,
}

#[wasm_bindgen]
impl ConwayWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let random_seed = 0x9e37_79b9;
        Self {
            conway: Conway::new(random_seed),
        }
    }

    pub fn tick_interval_ms(&self) -> u32 {
        self.conway.tick_interval_ms()
    }

    pub fn press_key(&mut self, key: &str) -> String {
        let command = match key {
            "prev" => ConwayCommand::Previous,
            "next" => ConwayCommand::Next,
            "play_pause" => ConwayCommand::PlayPause,
            "cancel" => ConwayCommand::Cancel,
            "speed_up" => ConwayCommand::SpeedUp,
            "speed_down" => ConwayCommand::SpeedDown,
            "mode" => ConwayCommand::Mode,
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                ConwayCommand::Pattern((key.as_bytes()[0] - b'0') as usize)
            }
            _ => return "unknown".into(),
        };
        self.conway.command(command).as_str().into()
    }

    pub fn tick(&mut self) -> String {
        self.conway.tick().as_str().into()
    }

    pub fn render_png(&mut self) -> Result<Vec<u8>, JsValue> {
        self.render_png_with_max_dimension(DEFAULT_MAX_DIMENSION)
    }

    pub fn render_png_with_max_dimension(
        &mut self,
        max_dimension: u32,
    ) -> Result<Vec<u8>, JsValue> {
        let frame = self.conway.frame();
        frame
            .to_png_bytes(max_dimension)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

impl Default for ConwayWeb {
    fn default() -> Self {
        Self::new()
    }
}
