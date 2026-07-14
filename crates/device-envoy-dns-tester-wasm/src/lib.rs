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
    dns::{DnsResult, DnsRuntime},
    flash_block::FlashBlock as _,
    wasm::{
        CydSimulatorControlWasm, CydSimulatorWasm, CydWasm, FlashBlockWasm, next_animation_frame,
    },
};
use device_envoy_examples_core::dns_tester::{
    Error as CoreError, Exit as CoreExit, UiError as CoreUiError,
    display_orientation_for_calibration, dns_tester, dns_tester_splash,
    orientation_after_calibration,
};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const DNS_HOSTNAME: &str = "example.com";
const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white

#[wasm_bindgen]
pub struct DnsTesterWeb {
    canvas: HtmlCanvasElement,
    exit: Rc<Cell<Option<CoreExit>>>,
    failed: Rc<Cell<bool>>,
    state: RefCell<DnsTesterState>,
}

struct DnsTesterState {
    wifi_flash_block: FlashBlockWasm,
    calibration_flash_block: FlashBlockWasm,
    orientation_flash_block: FlashBlockWasm,
    simulator_control: Option<CydSimulatorControlWasm>,
    orientation: Orientation,
    hostname: &'static str,
}

#[wasm_bindgen]
impl DnsTesterWeb {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<DnsTesterWeb, JsValue> {
        Ok(Self {
            canvas,
            exit: Rc::new(Cell::new(None)),
            failed: Rc::new(Cell::new(false)),
            state: RefCell::new(DnsTesterState {
                wifi_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/wifi")
                    .map_err(|error| JsValue::from_str(&format!("Wi-Fi flash: {error:?}")))?,
                calibration_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/calibration")
                    .map_err(|error| JsValue::from_str(&format!("Calibration flash: {error:?}")))?,
                orientation_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/orientation")
                    .map_err(|error| JsValue::from_str(&format!("Orientation flash: {error:?}")))?,
                simulator_control: None,
                orientation: Orientation::Landscape,
                hostname: DNS_HOSTNAME,
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

        let simulator = CydSimulatorWasm::new_with_style(
            self.canvas.clone(),
            orientation,
            BACKGROUND,
            FOREGROUND,
            &FONT_6X10,
        )?;
        let (device, mut button, simulator_control) = simulator.into_parts();
        state.simulator_control = Some(simulator_control);
        let (mut display, uncalibrated_touch) = device.parts_uncalibrated();
        let (touch, outcome) = ensure_calibration(
            &mut display,
            uncalibrated_touch,
            &mut state.calibration_flash_block,
            &mut button,
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
        let (mut display, touch) = if outcome.was_saved() {
            let dashboard_simulator = CydSimulatorWasm::new_with_style(
                self.canvas.clone(),
                saved_orientation,
                BACKGROUND,
                FOREGROUND,
                &FONT_6X10,
            )?;
            let (dashboard_device, dashboard_button, dashboard_control) =
                dashboard_simulator.into_parts();
            button = dashboard_button;
            state.simulator_control = Some(dashboard_control);
            let (display, uncalibrated_touch) = dashboard_device.parts_uncalibrated();
            (
                display,
                uncalibrated_touch.calibrate(outcome.calibration_config()),
            )
        } else {
            (display, touch)
        };
        dns_tester_splash(&mut display, state.orientation)
            .await
            .map_err(|error| JsValue::from_str(&format!("Splash: {error:?}")))?;
        for _ in 0..60 {
            next_animation_frame().await;
        }
        let mut device = CydWasm::from_parts(display, touch);
        let exit = self.exit.clone();
        let failed = self.failed.clone();
        let hostname = state.hostname;
        let mut dns = DnsRuntime::new(hostname, async || {
            Ok::<DnsResult, core::convert::Infallible>(DnsResult {
                succeeded: true,
                latency_millis: 12,
            })
        });
        drop(state);
        wasm_bindgen_futures::spawn_local(async move {
            match dns_tester(&mut device, &mut button, &mut dns).await {
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
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.touch_down(x, y);
        }
    }

    pub fn touch_move(&self, x: f32, y: f32) {
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.touch_move(x, y);
        }
    }

    pub fn touch_up(&self) {
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.touch_up();
        }
    }

    pub fn boot_down(&self) {
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.boot_down();
        }
    }

    pub fn boot_up(&self) {
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.boot_up();
        }
    }

    pub fn take_exit(&self) -> String {
        match self.exit.take() {
            Some(CoreExit::Calibrate) => "recalibrate".into(),
            Some(CoreExit::ResetWifi) => "wifi".into(),
            Some(CoreExit::Reorientate(next_orientation)) => {
                let save_result = self
                    .state
                    .borrow_mut()
                    .orientation_flash_block
                    .save(&next_orientation);
                match save_result {
                    Ok(()) => {
                        self.state.borrow_mut().orientation = next_orientation;
                        self.canvas.set_width(next_orientation.width());
                        self.canvas.set_height(next_orientation.height());
                        "orientation".into()
                    }
                    Err(_) => {
                        self.failed.set(true);
                        "runtime error".into()
                    }
                }
            }
            None if self.failed.get() => "runtime error".into(),
            None => "idle".into(),
        }
    }

    pub async fn reboot(&self) -> Result<(), JsValue> {
        self.start().await
    }

    /// Present the simulated CYD in landscape while touch calibration runs.
    pub fn prepare_calibration_landscape(&self) {
        if let Some(control) = self.state.borrow().simulator_control.as_ref() {
            control.reset_transient_state();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::geometry::Point;

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
                let mapped_point = match orientation {
                    Orientation::Landscape => point,
                    Orientation::Portrait => Point::new(319 - point.y, point.x),
                    Orientation::LandscapeInverted => Point::new(319 - point.x, 239 - point.y),
                    Orientation::PortraitInverted => Point::new(point.y, 239 - point.x),
                };
                assert_eq!(orientation.map_landscape_point(mapped_point), point);
            }
        }
    }
}
