use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use device_envoy_core::{
    cyd::display::Orientation,
    dns::{Addresses, Dns, IpAddress},
    flash_block::FlashBlock as _,
    wasm::{
        CydSimulatorControlWasm, CydSimulatorWasm, FlashBlockWasm, SimulatorNoticeDisposition,
        SimulatorNoticeRequest, WifiConnectEvent, WifiConnectOutcome, next_animation_frame,
        simulate_wifi_connect, simulator_notice_disposition,
    },
};
use device_envoy_examples_core::dns_tester::{
    Error as CoreError, Exit as CoreExit, UiError as CoreUiError, UiNotice, render_notice, run,
};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white

#[wasm_bindgen]
pub struct DnsTesterWeb {
    canvas: HtmlCanvasElement,
    exit: Rc<Cell<Option<CoreExit>>>,
    failed: Rc<Cell<bool>>,
    pending_notice: Cell<Option<SimulatorNoticeRequest>>,
    orientation: Cell<Orientation>,
    simulator_control: RefCell<Option<CydSimulatorControlWasm>>,
    state: RefCell<DnsTesterState>,
}

struct DnsTesterState {
    wifi_flash_block: FlashBlockWasm,
    calibration_flash_block: FlashBlockWasm,
    orientation_flash_block: FlashBlockWasm,
    orientation: Orientation,
}

struct MockDns;

impl Dns for MockDns {
    type Error = core::convert::Infallible;

    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        Ok([IpAddress::Ipv4([127, 0, 0, 1].into())]
            .into_iter()
            .collect())
    }
}

#[wasm_bindgen]
impl DnsTesterWeb {
    #[wasm_bindgen(constructor)] // todo000 Is this pretty code?
    pub fn new(canvas: HtmlCanvasElement) -> Result<DnsTesterWeb, JsValue> {
        Ok(Self {
            canvas,
            exit: Rc::new(Cell::new(None)),
            failed: Rc::new(Cell::new(false)),
            pending_notice: Cell::new(None),
            orientation: Cell::new(Orientation::Landscape),
            simulator_control: RefCell::new(None),
            state: RefCell::new(DnsTesterState {
                wifi_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/wifi")
                    .map_err(|error| JsValue::from_str(&format!("Wi-Fi flash: {error:?}")))?,
                calibration_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/calibration")
                    .map_err(|error| JsValue::from_str(&format!("Calibration flash: {error:?}")))?,
                orientation_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/orientation")
                    .map_err(|error| JsValue::from_str(&format!("Orientation flash: {error:?}")))?,
                orientation: Orientation::Landscape,
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
        let orientation = saved_orientation;
        self.orientation.set(orientation);
        state.orientation = orientation;
        self.canvas.set_width(orientation.width());
        self.canvas.set_height(orientation.height());

        // todo000 improve all these names here and elsewhere.
        let simulator = CydSimulatorWasm::new_with_style(
            self.canvas.clone(),
            orientation,
            BACKGROUND,
            FOREGROUND,
            &FONT_6X10,
        )?;
        let (mut device, mut button, simulator_control) = simulator.into_parts();
        *self.simulator_control.borrow_mut() = Some(simulator_control);
        // TODO0000 Consider using the core CYD splash helper here too.
        let mut display = device.display();
        render_notice(&mut display, state.orientation, UiNotice::Splash)
            .await
            .map_err(|error| JsValue::from_str(&format!("Splash: {error:?}")))?;
        for _ in 0..60 {
            //todo000 what does this mean?
            next_animation_frame().await;
        }

        let wifi_outcome = simulate_wifi_connect(&mut button, async |event| {
            let (notice_request, notice) = match event {
                WifiConnectEvent::CaptivePortalReady => {
                    (SimulatorNoticeRequest::wifi_setup(), UiNotice::WifiSetup)
                }
                WifiConnectEvent::Connecting { .. } => (
                    SimulatorNoticeRequest::wifi_connecting(),
                    UiNotice::WifiConnecting,
                ),
                WifiConnectEvent::ConnectionFailed => (
                    SimulatorNoticeRequest::wifi_unavailable(),
                    UiNotice::WifiUnavailable,
                ),
            };
            if matches!(
                self.request_notice(notice_request),
                SimulatorNoticeDisposition::Terminate
            ) {
                return Ok(());
            }
            // TODO0000 Consider adding a core helper for this notice too.
            let mut display = device.display();
            render_notice(&mut display, state.orientation, notice)
                .await
                .map_err(|error| JsValue::from_str(&format!("Wi-Fi notice: {error:?}")))
        })
        .await?;
        if matches!(wifi_outcome, WifiConnectOutcome::ResetRequested) {
            self.exit.set(Some(CoreExit::ResetWifi));
            return Ok(());
        }

        let exit = self.exit.clone();
        let failed = self.failed.clone();
        let mut dns = MockDns;
        drop(state);
        wasm_bindgen_futures::spawn_local(async move {
            match run(&mut device, &mut button, &mut dns).await {
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
        if let Some(control) = self.simulator_control.borrow().as_ref() {
            control.touch_down(x, y);
        }
    }

    pub fn touch_move(&self, x: f32, y: f32) {
        if let Some(control) = self.simulator_control.borrow().as_ref() {
            control.touch_move(x, y);
        }
    }

    pub fn touch_up(&self) {
        if let Some(control) = self.simulator_control.borrow().as_ref() {
            control.touch_up();
        }
    }

    pub fn boot_down(&self) {
        if let Some(control) = self.simulator_control.borrow().as_ref() {
            control.boot_down();
        }
    }

    pub fn boot_up(&self) {
        if let Some(control) = self.simulator_control.borrow().as_ref() {
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
                        self.orientation.set(next_orientation);
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

    /// Take the next typed browser notice identifier, if one was requested.
    pub fn take_notice(&self) -> String {
        self.pending_notice
            .take()
            .map(|request| request.id.into())
            .unwrap_or_default()
    }

    pub async fn reboot(&self) -> Result<(), JsValue> {
        self.start().await
    }

    /// Present the simulated CYD in landscape while touch calibration runs.
    pub fn prepare_calibration_landscape(&self) {
        if let Some(control) = self.simulator_control.borrow().as_ref() {
            control.reset_transient_state();
        }
        self.state.borrow_mut().orientation = Orientation::Landscape;
        self.orientation.set(Orientation::Landscape);
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

    /// Clear only touch calibration storage before a recalibration restart.
    pub fn clear_calibration(&self) -> Result<(), JsValue> {
        self.state
            .borrow_mut()
            .calibration_flash_block
            .clear()
            .map_err(|error| JsValue::from_str(&format!("Calibration clear: {error:?}")))
    }
}

impl DnsTesterWeb {
    fn request_notice(&self, request: SimulatorNoticeRequest) -> SimulatorNoticeDisposition {
        let disposition = simulator_notice_disposition(request);
        self.pending_notice.set(Some(request));
        if matches!(disposition, SimulatorNoticeDisposition::Terminate) {
            self.failed.set(true);
        }
        disposition
    }

    fn orientation(&self) -> Orientation {
        self.orientation.get()
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
