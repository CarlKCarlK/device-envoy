use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use device_envoy_core::{
    cyd::{
        CydParts as _, CydTouchUncalibrated as _,
        display::Orientation,
        touch::calibration::{CalibrationConfig, ensure_calibration},
    },
    dns_lookup::{DnsLookupFn, DnsLookupResult},
    flash_block::FlashBlock as _,
    wasm::{ButtonWasmSource, CydTouchWasmSource, CydWasm, FlashBlockWasm},
};
use device_envoy_examples_core::dns_tester::{
    Error as CoreError, Exit as CoreExit, UiError as CoreUiError,
    display_orientation_for_calibration, dns_tester, orientation_after_calibration,
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
    exit: Rc<Cell<Option<CoreExit>>>,
    failed: Rc<Cell<bool>>,
    state: RefCell<DnsTesterState>,
}

struct DnsTesterState {
    wifi_flash_block: FlashBlockWasm,
    calibration_flash_block: FlashBlockWasm,
    orientation_flash_block: FlashBlockWasm,
    orientation: Orientation,
    target: &'static str,
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
            exit: Rc::new(Cell::new(None)),
            failed: Rc::new(Cell::new(false)),
            state: RefCell::new(DnsTesterState {
                wifi_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/wifi")
                    .map_err(|error| JsValue::from_str(&format!("Wi-Fi flash: {error:?}")))?,
                calibration_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/calibration")
                    .map_err(|error| JsValue::from_str(&format!("Calibration flash: {error:?}")))?,
                orientation_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/orientation")
                    .map_err(|error| JsValue::from_str(&format!("Orientation flash: {error:?}")))?,
                orientation: Orientation::Landscape,
                target: DNS_HOSTNAME,
            }),
        })
    }

    pub async fn start(&self) -> Result<(), JsValue> {
        self.exit.set(None);
        self.failed.set(false);
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
        let orientation =
            display_orientation_for_calibration(saved_orientation, calibration_config.is_some());
        state.orientation = orientation;
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
        if outcome.was_saved() {
            // Calibration is always completed in landscape. Restore the saved
            // dashboard orientation as soon as calibration has been persisted.
            state.orientation = orientation_after_calibration(saved_orientation);
            self.canvas.set_width(saved_orientation.width());
            self.canvas.set_height(saved_orientation.height());
        }
        let (display, touch) = if outcome.was_saved() {
            let dashboard_device = CydWasm::new(
                self.context.clone(),
                saved_orientation,
                BACKGROUND,
                FOREGROUND,
                &FONT_6X10,
                self.touch_source.clone(),
            );
            let (display, uncalibrated_touch) = dashboard_device.parts_uncalibrated();
            (
                display,
                uncalibrated_touch.calibrate(outcome.calibration_config()),
            )
        } else {
            (display, touch)
        };
        let mut device = CydWasm::from_parts(display, touch);
        let exit = self.exit.clone();
        let failed = self.failed.clone();
        let target = state.target;
        let mut button = self.button_source.button();
        let mut dns_lookup = DnsLookupFn(async |_hostname: &str| {
            Ok::<DnsLookupResult, core::convert::Infallible>(DnsLookupResult {
                succeeded: true,
                latency_millis: 12,
            })
        });
        drop(state);
        wasm_bindgen_futures::spawn_local(async move {
            match dns_tester(&mut device, &mut button, target, &mut dns_lookup).await {
                Ok(exit_value) => exit.set(Some(exit_value)),
                Err(CoreError::Display(CoreUiError::Text(_))) => failed.set(true),
                Err(CoreError::Display(CoreUiError::Display(error))) => match error {},
                Err(CoreError::Touch(error)) => match error {},
                Err(CoreError::Dns(error)) => match error {},
            }
        });
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

    pub fn take_exit(&self) -> String {
        match self.exit.take() {
            Some(CoreExit::CalibrationRequested) => "recalibrate".into(),
            Some(CoreExit::WifiResetRequested) => "wifi".into(),
            Some(CoreExit::OrientationChanged(_)) => "orientation".into(),
            None if self.failed.get() => "runtime error".into(),
            None => "idle".into(),
        }
    }

    pub async fn reboot(&self) -> Result<(), JsValue> {
        self.start().await
    }

    /// Present the simulated CYD in landscape while touch calibration runs.
    pub fn prepare_calibration_landscape(&self) {
        self.state.borrow_mut().orientation = Orientation::Landscape;
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
    fn orientation(&self) -> Orientation {
        self.state.borrow().orientation
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
                assert_eq!(orientation.map_landscape_point(landscape_point), point);
            }
        }
    }
}
