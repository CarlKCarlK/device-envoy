use std::cell::{Cell, RefCell};

use device_envoy_core::{
    UnwrapInfallible as _,
    button::Button as _,
    cyd::{
        CydTouch as _, CydTouchUncalibrated as _,
        display::Orientation,
        touch::calibration::{CalibrationConfig, ensure_calibration},
    },
    flash_block::FlashBlock as _,
    wasm::{
        ButtonWasmSource, CydDisplayWasm, CydTouchWasm, CydTouchWasmSource, CydWasm, FlashBlockWasm,
    },
};
use device_envoy_examples_core::dns_tester::{
    DnsResult, DnsTesterAction, DnsTesterApp, DnsTesterInput, DnsTesterUiError, render_app,
};
use embedded_graphics::{geometry::Point, mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const DNS_HOSTNAME: &str = "example.com";
const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white

#[wasm_bindgen]
pub struct DnsTesterWeb {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    touch_source: CydTouchWasmSource,
    button_source: ButtonWasmSource,
    ready: Cell<bool>,
    state: RefCell<DnsTesterState>,
}

struct DnsTesterState {
    display: Option<CydDisplayWasm>,
    touch: Option<CydTouchWasm>,
    wifi_flash_block: FlashBlockWasm,
    calibration_flash_block: FlashBlockWasm,
    orientation_flash_block: FlashBlockWasm,
    app: DnsTesterApp,
    button_was_pressed: bool,
}

#[wasm_bindgen]
impl DnsTesterWeb {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<DnsTesterWeb, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2D canvas context unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(Self {
            canvas,
            context,
            touch_source: CydTouchWasmSource::new(),
            button_source: ButtonWasmSource::new(),
            ready: Cell::new(false),
            state: RefCell::new(DnsTesterState {
                display: None,
                touch: None,
                wifi_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/wifi")
                    .map_err(|error| JsValue::from_str(&format!("Wi-Fi flash: {error:?}")))?,
                calibration_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/calibration")
                    .map_err(|error| JsValue::from_str(&format!("Calibration flash: {error:?}")))?,
                orientation_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/orientation")
                    .map_err(|error| JsValue::from_str(&format!("Orientation flash: {error:?}")))?,
                app: DnsTesterApp::new(DNS_HOSTNAME, Orientation::Landscape),
                button_was_pressed: false,
            }),
        })
    }

    pub async fn start(&self) -> Result<(), JsValue> {
        self.ready.set(false);
        let mut state = self.state.borrow_mut();
        let saved_orientation = state
            .orientation_flash_block
            .load::<Orientation>()
            .map_err(|error| JsValue::from_str(&format!("Orientation load: {error:?}")))?
            .unwrap_or(Orientation::Landscape);
        let calibration_config = match state.calibration_flash_block.load::<CalibrationConfig>() {
            Ok(calibration_config) => calibration_config,
            Err(_) => None,
        };
        let orientation = DnsTesterApp::display_orientation_for_calibration(
            saved_orientation,
            calibration_config.is_some(),
        );
        state.app.set_orientation(orientation);
        self.canvas.set_width(orientation.width());
        self.canvas.set_height(orientation.height());

        let device = CydWasm::new(
            self.context.clone(),
            orientation,
            BACKGROUND,
            FOREGROUND,
            &FONT_6X10,
            self.touch_source.clone(),
        );
        let (mut display, uncalibrated_touch) = device.parts_uncalibrated();
        let (touch, outcome) = ensure_calibration(
            &mut display,
            uncalibrated_touch,
            &mut state.calibration_flash_block,
            &mut self.button_source.button(),
            Some("Touch calibrated"),
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("Calibration: {error:?}")))?;
        state.display = Some(display);
        state.touch = Some(touch);

        if outcome.was_saved() {
            // Calibration is always completed in landscape. Restore the saved
            // dashboard orientation as soon as calibration has been persisted.
            state
                .app
                .set_orientation(DnsTesterApp::orientation_after_calibration(
                    saved_orientation,
                ));
            self.canvas.set_width(saved_orientation.width());
            self.canvas.set_height(saved_orientation.height());
            self.rebuild_display(&mut state, Some(outcome.calibration_config()));
        }
        state.app.input(DnsTesterInput::WifiReady);
        drop(state);
        self.ready.set(true);
        self.present().await
    }

    pub async fn present(&self) -> Result<(), JsValue> {
        if !self.ready.get() {
            return Ok(());
        }
        let mut state = self.state.borrow_mut();
        let app = state.app;
        let Some(display) = &mut state.display else {
            return Ok(());
        };
        render_app(display, &app)
            .await
            .map_err(|error| match error {
                DnsTesterUiError::Text(_) => JsValue::from_str("DNS tester text formatting failed"),
                DnsTesterUiError::Display(error) => match error {},
            })?;
        Ok(())
    }

    pub fn touch_down(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation(), Point::new(x as i32, y as i32));
        self.touch_source.touch_down(point.x as f32, point.y as f32);
    }

    pub fn touch_move(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation(), Point::new(x as i32, y as i32));
        self.touch_source.touch_move(point.x as f32, point.y as f32);
    }

    pub fn touch_up(&self) {
        self.touch_source.touch_up();
    }

    pub fn boot_down(&self) {
        self.button_source.press();
    }

    pub fn boot_up(&self) {
        self.button_source.release();
    }

    pub fn tick(&self) -> String {
        if !self.ready.get() {
            return "starting".into();
        }
        let mut state = self.state.borrow_mut();
        let is_pressed = self.button_source.button().is_pressed();
        if is_pressed && !state.button_was_pressed {
            state.button_was_pressed = true;
            let action = state.app.input(DnsTesterInput::Boot);
            return self.apply_action(&mut state, action);
        }
        state.button_was_pressed = is_pressed;

        let Some(touch) = &mut state.touch else {
            return "starting".into();
        };
        let Some(event) = touch.read().unwrap_infallible() else {
            return "idle".into();
        };
        let event = match event {
            device_envoy_core::cyd::touch::TouchEvent::Down { point } => {
                device_envoy_core::cyd::touch::TouchEvent::Down {
                    point: map_to_orientation(state.app.orientation(), point),
                }
            }
            event => event,
        };
        let action = state.app.input(DnsTesterInput::Touch(event));
        if matches!(action, DnsTesterAction::StartDnsLookup) {
            state.app.input(DnsTesterInput::DnsFinished(DnsResult {
                succeeded: true,
                latency_millis: 12,
            }));
            return format!("DNS success: {DNS_HOSTNAME} (12ms)");
        }
        self.apply_action(&mut state, action)
    }

    pub async fn reboot(&self) -> Result<(), JsValue> {
        self.start().await
    }

    /// Present the simulated CYD in landscape while touch calibration runs.
    pub fn prepare_calibration_landscape(&self) {
        self.state
            .borrow_mut()
            .app
            .set_orientation(Orientation::Landscape);
        self.canvas.set_width(Orientation::Landscape.width());
        self.canvas.set_height(Orientation::Landscape.height());
    }

    /// Whether the current simulated display orientation is upside down.
    pub fn orientation_is_inverted(&self) -> bool {
        matches!(
            self.orientation(),
            Orientation::LandscapeInverted | Orientation::PortraitInverted
        )
    }

    pub async fn clear_storage(&self) -> Result<(), JsValue> {
        let mut state = self.state.borrow_mut();
        state
            .wifi_flash_block
            .clear()
            .map_err(|error| JsValue::from_str(&format!("Wi-Fi clear: {error:?}")))?;
        state
            .calibration_flash_block
            .clear()
            .map_err(|error| JsValue::from_str(&format!("Calibration clear: {error:?}")))?;
        state
            .orientation_flash_block
            .clear()
            .map_err(|error| JsValue::from_str(&format!("Orientation clear: {error:?}")))?;
        drop(state);
        self.start().await
    }
}

impl DnsTesterWeb {
    fn rebuild_display(
        &self,
        state: &mut DnsTesterState,
        calibration_config: Option<CalibrationConfig>,
    ) {
        let device = CydWasm::new(
            self.context.clone(),
            self.orientation(),
            BACKGROUND,
            FOREGROUND,
            &FONT_6X10,
            self.touch_source.clone(),
        );
        let (display, uncalibrated_touch) = device.parts_uncalibrated();
        let touch = match calibration_config {
            Some(calibration_config) => uncalibrated_touch.calibrate(calibration_config),
            None => {
                uncalibrated_touch.calibrate(CalibrationConfig::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0))
            }
        };
        state.display = Some(display);
        state.touch = Some(touch);
    }

    fn action(message: &str) -> String {
        message.into()
    }
}

fn map_to_orientation(orientation: Orientation, point: Point) -> Point {
    orientation.map_landscape_point(point)
}

impl DnsTesterWeb {
    fn orientation(&self) -> Orientation {
        self.state.borrow().app.orientation()
    }

    fn apply_action(&self, state: &mut DnsTesterState, action: DnsTesterAction) -> String {
        match action {
            DnsTesterAction::None => "idle".into(),
            DnsTesterAction::StartDnsLookup => "starting DNS lookup".into(),
            DnsTesterAction::ClearCalibrationAndRestart => {
                if state.calibration_flash_block.clear().is_err() {
                    return Self::action("storage error");
                }
                Self::action("recalibrate")
            }
            DnsTesterAction::ResetWifiAndRestart => Self::action("wifi"),
            DnsTesterAction::SaveOrientationAndRestart(orientation) => {
                if state.orientation_flash_block.save(&orientation).is_err() {
                    return Self::action("storage error");
                }
                Self::action("orientation")
            }
        }
    }
}

fn map_to_landscape(orientation: Orientation, point: Point) -> Point {
    match orientation {
        Orientation::Landscape => point,
        Orientation::Portrait => Point::new(319 - point.y, point.x),
        Orientation::LandscapeInverted => Point::new(319 - point.x, 239 - point.y),
        Orientation::PortraitInverted => Point::new(point.y, 239 - point.x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn control_hitboxes_round_trip_through_every_orientation() {
        for (orientation, calibration, wifi, rotate) in [
            (
                Orientation::Landscape,
                Point::new(60, 216),
                Point::new(160, 216),
                Point::new(260, 216),
            ),
            (
                Orientation::Portrait,
                Point::new(46, 294),
                Point::new(120, 294),
                Point::new(193, 294),
            ),
            (
                Orientation::LandscapeInverted,
                Point::new(60, 216),
                Point::new(160, 216),
                Point::new(260, 216),
            ),
            (
                Orientation::PortraitInverted,
                Point::new(46, 294),
                Point::new(120, 294),
                Point::new(193, 294),
            ),
        ] {
            for point in [calibration, wifi, rotate] {
                let landscape_point = map_to_landscape(orientation, point);
                assert_eq!(map_to_orientation(orientation, landscape_point), point);
            }
        }
    }
}
