use std::cell::{Cell, RefCell};

use device_envoy_core::{
    UnwrapInfallible as _,
    button::Button as _,
    cyd::{
        CydTouch as _, CydTouchUncalibrated as _,
        display::Orientation,
        touch::{
            TouchEvent,
            calibration::{CalibrationConfig, ensure_calibration},
        },
    },
    dns_tester::{DnsTesterUiError, DnsTesterUiNotice, DnsTesterUiState, render, render_notice},
    flash_block::FlashBlock as _,
    wasm::{
        ButtonWasmSource, CydDisplayWasm, CydTouchWasm, CydTouchWasmSource, CydWasm, FlashBlockWasm,
    },
};
use embedded_graphics::{
    geometry::{Point, Size},
    mono_font::ascii::FONT_6X10,
    pixelcolor::Rgb888,
    primitives::Rectangle,
};
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
    orientation: Cell<Orientation>,
    ready: Cell<bool>,
    state: RefCell<DnsTesterState>,
}

struct DnsTesterState {
    display: Option<CydDisplayWasm>,
    touch: Option<CydTouchWasm>,
    wifi_flash_block: FlashBlockWasm,
    calibration_flash_block: FlashBlockWasm,
    orientation_flash_block: FlashBlockWasm,
    taps: u32,
    successes: u32,
    failures: u32,
    last_latency_millis: u64,
    button_was_pressed: bool,
    screen: Screen,
}

#[derive(Clone, Copy)]
enum Screen {
    Tester,
    WifiUnavailable,
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
            orientation: Cell::new(Orientation::Landscape),
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
                taps: 0,
                successes: 0,
                failures: 0,
                last_latency_millis: 0,
                button_was_pressed: false,
                screen: Screen::Tester,
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
        let orientation = if calibration_config.is_some() {
            saved_orientation
        } else {
            Orientation::Landscape
        };
        self.orientation.set(orientation);
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
            self.orientation.set(saved_orientation);
            self.canvas.set_width(saved_orientation.width());
            self.canvas.set_height(saved_orientation.height());
            self.rebuild_display(&mut state, Some(outcome.calibration_config()));
        }
        drop(state);
        self.ready.set(true);
        self.present().await
    }

    pub async fn present(&self) -> Result<(), JsValue> {
        if !self.ready.get() {
            return Ok(());
        }
        let mut state = self.state.borrow_mut();
        let taps = state.taps;
        let successes = state.successes;
        let failures = state.failures;
        let last_latency_millis = state.last_latency_millis;
        let screen = state.screen;
        let Some(display) = &mut state.display else {
            return Ok(());
        };
        match screen {
            Screen::Tester => {
                render(
                    display,
                    self.orientation.get(),
                    DnsTesterUiState {
                        target: DNS_HOSTNAME,
                        queries: taps,
                        successes,
                        failures,
                        last_latency_millis,
                    },
                )
                .await
            }
            Screen::WifiUnavailable => {
                render_notice(
                    display,
                    self.orientation.get(),
                    DnsTesterUiNotice::WifiUnavailable,
                )
                .await
            }
        }
        .map_err(|error| match error {
            DnsTesterUiError::Text(_) => JsValue::from_str("DNS tester text formatting failed"),
            DnsTesterUiError::Display(error) => match error {},
        })?;
        Ok(())
    }

    pub fn touch_down(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation.get(), Point::new(x as i32, y as i32));
        self.touch_source.touch_down(point.x as f32, point.y as f32);
    }

    pub fn touch_move(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation.get(), Point::new(x as i32, y as i32));
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
            if state.calibration_flash_block.clear().is_err() {
                return Self::action("storage error");
            }
            return Self::action("recalibrate");
        }
        state.button_was_pressed = is_pressed;

        let Some(touch) = &mut state.touch else {
            return "starting".into();
        };
        let point = loop {
            match touch.read().unwrap_infallible() {
                Some(TouchEvent::Down { point }) => break point,
                Some(TouchEvent::Move { .. } | TouchEvent::Up) => {}
                None => return "idle".into(),
            }
        };
        let orientation = self.orientation.get();
        let point = map_to_orientation(orientation, point);
        if let Some(control) = control_at(point, orientation.size()) {
            return match control {
                Control::Orientation => {
                    let next_orientation = self.orientation.get().next();
                    if state
                        .orientation_flash_block
                        .save(&next_orientation)
                        .is_err()
                    {
                        return Self::action("storage error");
                    }
                    Self::action("orientation")
                }
                Control::Calibration => {
                    if state.calibration_flash_block.clear().is_err() {
                        return Self::action("storage error");
                    }
                    Self::action("recalibrate")
                }
                Control::Wifi => {
                    state.screen = Screen::WifiUnavailable;
                    Self::action("wifi")
                }
            };
        }

        state.screen = Screen::Tester;
        state.taps += 1;
        state.successes += 1;
        state.last_latency_millis = 12;
        format!(
            "DNS success: {DNS_HOSTNAME} ({}ms)",
            state.last_latency_millis
        )
    }

    pub async fn reboot(&self) -> Result<(), JsValue> {
        self.start().await
    }

    /// Present the simulated CYD in landscape while touch calibration runs.
    pub fn prepare_calibration_landscape(&self) {
        self.orientation.set(Orientation::Landscape);
        self.canvas.set_width(Orientation::Landscape.width());
        self.canvas.set_height(Orientation::Landscape.height());
    }

    /// Whether the current simulated display orientation is upside down.
    pub fn orientation_is_inverted(&self) -> bool {
        matches!(
            self.orientation.get(),
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
            self.orientation.get(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Control {
    Orientation,
    Calibration,
    Wifi,
}

fn control_rectangle(control: Control, size: Size) -> Rectangle {
    match (control, size.width > size.height) {
        (Control::Calibration, true) => Rectangle::new(Point::new(10, 202), Size::new(100, 28)),
        (Control::Wifi, true) => Rectangle::new(Point::new(110, 202), Size::new(100, 28)),
        (Control::Orientation, true) => Rectangle::new(Point::new(210, 202), Size::new(100, 28)),
        (Control::Calibration, false) => Rectangle::new(Point::new(10, 276), Size::new(73, 36)),
        (Control::Wifi, false) => Rectangle::new(Point::new(83, 276), Size::new(74, 36)),
        (Control::Orientation, false) => Rectangle::new(Point::new(157, 276), Size::new(73, 36)),
    }
}

fn control_at(point: Point, size: Size) -> Option<Control> {
    [Control::Calibration, Control::Wifi, Control::Orientation]
        .into_iter()
        .find(|control| control_rectangle(*control, size).contains(point))
}

fn map_to_orientation(orientation: Orientation, point: Point) -> Point {
    orientation.map_landscape_point(point)
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
            for (point, expected) in [
                (calibration, Control::Calibration),
                (wifi, Control::Wifi),
                (rotate, Control::Orientation),
            ] {
                let landscape_point = map_to_landscape(orientation, point);
                assert_eq!(map_to_orientation(orientation, landscape_point), point);
                assert_eq!(control_at(point, orientation.size()), Some(expected));
            }
        }
    }
}
