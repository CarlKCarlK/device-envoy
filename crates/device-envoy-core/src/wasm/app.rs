//! A device abstraction for complete CYD browser applications.

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

use embassy_futures::select::{Either, select};
use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{HtmlCanvasElement, window};

use super::{
    ButtonWasm, CydDisplayWasm, CydSimulatorControlWasm, CydSimulatorWasm, FlashBlockWasm,
};
use crate::cyd::display::Orientation;
use crate::flash_block::FlashBlock as _;

/// Configuration shared by the CYD web supervisor and one application.
#[derive(Clone, Copy)]
pub struct CydWebAppConfig {
    /// Stable prefix for framework-owned browser storage.
    pub storage_namespace: &'static str,
    /// Orientation used when no saved orientation exists.
    pub initial_orientation: Orientation,
    /// Application display background.
    pub background: Rgb888,
    /// Application display foreground.
    pub foreground: Rgb888,
    /// Application display font.
    pub font: &'static MonoFont<'static>,
}

impl CydWebAppConfig {
    /// Construct a CYD web application configuration.
    pub const fn new(
        storage_namespace: &'static str,
        initial_orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self {
        Self {
            storage_namespace,
            initial_orientation,
            background,
            foreground,
            font,
        }
    }
}

/// A typed request from an application to the CYD web supervisor.
pub enum CydWebCommand {
    /// Reconstruct the current virtual device.
    Restart,
    /// Explain that browser touch calibration is unnecessary and restart.
    CalibrationNotNeeded,
    /// Reset simulated Wi-Fi state and reconstruct the virtual device.
    ResetWifi,
    /// Persist an orientation and reconstruct the virtual device.
    Reorientate(Orientation),
    /// Stop the supervisor without a fatal notice.
    Stop,
}

/// Severity for a notice consumed by the shared browser shell.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CydWebNoticeSeverity {
    /// Informational notice.
    Info,
    /// Recoverable warning.
    Warning,
    /// Fatal runtime failure.
    Fatal,
}

/// A typed browser notice with a stable, localizable identifier.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct CydWebNotice {
    id: String,
    severity: CydWebNoticeSeverity,
    detail: Option<String>,
}

impl CydWebNotice {
    fn new(id: impl Into<String>, severity: CydWebNoticeSeverity) -> Self {
        Self {
            id: id.into(),
            severity,
            detail: None,
        }
    }

    fn fatal(detail: String) -> Self {
        Self {
            id: "runtime-error".into(),
            severity: CydWebNoticeSeverity::Fatal,
            detail: Some(detail),
        }
    }
}

#[wasm_bindgen]
impl CydWebNotice {
    /// Return the stable notice identifier.
    #[wasm_bindgen(js_name = id)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Return the notice severity.
    #[wasm_bindgen(js_name = severity)]
    pub fn severity(&self) -> CydWebNoticeSeverity {
        self.severity
    }

    /// Return the formatted diagnostic, when this is a fatal runtime notice.
    #[wasm_bindgen(js_name = detail)]
    pub fn detail(&self) -> Option<String> {
        self.detail.clone()
    }
}

#[derive(Clone, Copy)]
enum HostRequest {
    Restart,
    ClearStorage,
}

struct LifecycleState {
    request: Option<HostRequest>,
    waker: Option<Waker>,
}

#[derive(Clone)]
struct LifecycleSignal {
    state: Rc<RefCell<LifecycleState>>,
}

impl LifecycleSignal {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(LifecycleState {
                request: None,
                waker: None,
            })),
        }
    }

    fn request(&self, request: HostRequest) {
        let waker = {
            let mut state = self.state.borrow_mut();
            state.request = Some(request);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    async fn wait(&self) -> HostRequest {
        LifecycleRequestFuture {
            signal: self.clone(),
        }
        .await
    }
}

struct LifecycleRequestFuture {
    signal: LifecycleSignal,
}

impl Future for LifecycleRequestFuture {
    type Output = HostRequest;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.signal.state.borrow_mut();
        if let Some(request) = state.request.take() {
            return Poll::Ready(request);
        }
        state.waker = Some(context.waker().clone());
        Poll::Pending
    }
}

struct SupervisorState {
    live_control: Option<CydSimulatorControlWasm>,
    notices: std::collections::VecDeque<CydWebNotice>,
    orientation: Orientation,
    stopped: bool,
}

/// Stable browser handle shared by every CYD web application.
#[wasm_bindgen]
pub struct CydWebAppHandle {
    state: Rc<RefCell<SupervisorState>>,
    lifecycle_signal: LifecycleSignal,
}

impl CydWebAppHandle {
    fn new(state: Rc<RefCell<SupervisorState>>, lifecycle_signal: LifecycleSignal) -> Self {
        Self {
            state,
            lifecycle_signal,
        }
    }

    fn with_control(&self, action: impl FnOnce(&CydSimulatorControlWasm)) {
        let state = self.state.borrow();
        if !state.stopped {
            if let Some(control) = state.live_control.as_ref() {
                action(control);
            }
        }
    }
}

#[wasm_bindgen]
impl CydWebAppHandle {
    /// Forward a pointer-down position in logical canvas coordinates.
    pub fn touch_down(&self, position_x: f32, position_y: f32) {
        self.with_control(|control| control.touch_down(position_x, position_y));
    }

    /// Forward a pointer-move position in logical canvas coordinates.
    pub fn touch_move(&self, position_x: f32, position_y: f32) {
        self.with_control(|control| control.touch_move(position_x, position_y));
    }

    /// Forward a pointer-up or pointer-cancel event.
    pub fn touch_up(&self) {
        self.with_control(CydSimulatorControlWasm::touch_up);
    }

    /// Forward a physical BOOT-button press.
    pub fn boot_down(&self) {
        self.with_control(CydSimulatorControlWasm::boot_down);
    }

    /// Forward a physical BOOT-button release.
    pub fn boot_up(&self) {
        self.with_control(CydSimulatorControlWasm::boot_up);
    }

    /// Return whether the current presentation is inverted.
    pub fn orientation_is_inverted(&self) -> bool {
        self.state
            .borrow()
            .live_control
            .as_ref()
            .is_some_and(CydSimulatorControlWasm::orientation_is_inverted)
    }

    /// Take the oldest pending typed notice.
    pub fn take_notice(&self) -> Option<CydWebNotice> {
        self.state.borrow_mut().notices.pop_front()
    }

    /// Request an orderly supervisor restart.
    pub fn request_restart(&self) {
        self.lifecycle_signal.request(HostRequest::Restart);
    }

    /// Clear framework storage and request an orderly supervisor restart.
    pub fn clear_storage_and_restart(&self) {
        self.lifecycle_signal.request(HostRequest::ClearStorage);
    }
}

/// Start a complete CYD web application under the shared supervisor.
pub fn start_cyd_web_app<Run, Error>(
    canvas_id: &str,
    config: CydWebAppConfig,
    inner_main: Run,
) -> Result<CydWebAppHandle, JsValue>
where
    Run: for<'a> AsyncFnMut(
            &'a mut super::CydWasm,
            &'a mut ButtonWasm,
        ) -> Result<CydWebCommand, Error>
        + 'static,
    Error: Debug + 'static,
{
    let canvas = canvas(canvas_id)?;
    let mut orientation_flash_block =
        FlashBlockWasm::new(&format!("{}/orientation", config.storage_namespace))
            .map_err(|error| JsValue::from_str(&format!("orientation storage: {error:?}")))?;
    let orientation = orientation_flash_block
        .load::<Orientation>()
        .map_err(|error| JsValue::from_str(&format!("orientation load: {error:?}")))?
        .unwrap_or(config.initial_orientation);
    let simulator = CydSimulatorWasm::new_with_style(
        canvas.clone(),
        orientation,
        config.background,
        config.foreground,
        config.font,
    )?;
    let (cyd, button, control) = simulator.into_parts();
    let state = Rc::new(RefCell::new(SupervisorState {
        live_control: Some(control),
        notices: std::collections::VecDeque::new(),
        orientation,
        stopped: false,
    }));
    let lifecycle_signal = LifecycleSignal::new();
    let handle = CydWebAppHandle::new(state.clone(), lifecycle_signal.clone());
    wasm_bindgen_futures::spawn_local(supervise(
        canvas,
        config,
        orientation_flash_block,
        state,
        lifecycle_signal,
        inner_main,
        Some((cyd, button)),
    ));
    Ok(handle)
}

/// Start a CYD web application that requires only a display and button input.
pub fn start_cyd_display_web_app<Run, Error>(
    canvas_id: &str,
    config: CydWebAppConfig,
    inner_main: Run,
) -> Result<CydWebAppHandle, JsValue>
where
    Run: for<'a> AsyncFnMut(
            &'a mut CydDisplayWasm,
            &'a mut ButtonWasm,
        ) -> Result<CydWebCommand, Error>
        + 'static,
    Error: Debug + 'static,
{
    let canvas = canvas(canvas_id)?;
    let mut orientation_flash_block =
        FlashBlockWasm::new(&format!("{}/orientation", config.storage_namespace))
            .map_err(|error| JsValue::from_str(&format!("orientation storage: {error:?}")))?;
    let orientation = orientation_flash_block
        .load::<Orientation>()
        .map_err(|error| JsValue::from_str(&format!("orientation load: {error:?}")))?
        .unwrap_or(config.initial_orientation);
    let simulator = CydSimulatorWasm::new_display_with_style(
        canvas.clone(),
        orientation,
        config.background,
        config.foreground,
        config.font,
    )?;
    let (display, button, control) = simulator.into_parts();
    let state = Rc::new(RefCell::new(SupervisorState {
        live_control: Some(control),
        notices: std::collections::VecDeque::new(),
        orientation,
        stopped: false,
    }));
    let lifecycle_signal = LifecycleSignal::new();
    let handle = CydWebAppHandle::new(state.clone(), lifecycle_signal.clone());
    wasm_bindgen_futures::spawn_local(supervise_display(
        canvas,
        config,
        orientation_flash_block,
        state,
        lifecycle_signal,
        inner_main,
        Some((display, button)),
    ));
    Ok(handle)
}

fn canvas(canvas_id: &str) -> Result<HtmlCanvasElement, JsValue> {
    let document = window()
        .ok_or_else(|| JsValue::from_str("browser window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas element unavailable"))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(Into::into)
}

async fn supervise<Run, Error>(
    canvas: HtmlCanvasElement,
    config: CydWebAppConfig,
    mut orientation_flash_block: FlashBlockWasm,
    state: Rc<RefCell<SupervisorState>>,
    lifecycle_signal: LifecycleSignal,
    mut inner_main: Run,
    initial_session: Option<(super::CydWasm, ButtonWasm)>,
) where
    Run: for<'a> AsyncFnMut(
            &'a mut super::CydWasm,
            &'a mut ButtonWasm,
        ) -> Result<CydWebCommand, Error>
        + 'static,
    Error: Debug + 'static,
{
    let mut session = initial_session;
    loop {
        let (mut cyd, mut button) = match session.take() {
            Some(session) => session,
            None => {
                let orientation = state.borrow().orientation;
                match CydSimulatorWasm::new_with_style(
                    canvas.clone(),
                    orientation,
                    config.background,
                    config.foreground,
                    config.font,
                ) {
                    Ok(simulator) => {
                        let (cyd, button, control) = simulator.into_parts();
                        replace_control(&state, control);
                        (cyd, button)
                    }
                    Err(error) => {
                        fatal(&state, format!("simulator construction failed: {error:?}"));
                        break;
                    }
                }
            }
        };

        let command = match select(inner_main(&mut cyd, &mut button), lifecycle_signal.wait()).await
        {
            Either::First(result) => match result {
                Ok(command) => command,
                Err(error) => {
                    fatal(&state, format!("application failed: {error:?}"));
                    break;
                }
            },
            Either::Second(request) => {
                match apply_host_request(request, &config, &mut orientation_flash_block, &state) {
                    Ok(()) => CydWebCommand::Restart,
                    Err(error) => {
                        fatal(&state, error);
                        break;
                    }
                }
            }
        };

        release_control(&state);
        match command {
            CydWebCommand::Stop => break,
            CydWebCommand::Restart => {}
            CydWebCommand::ResetWifi => {
                super::WifiSimulatorWasm::new(config.storage_namespace).reset();
                state.borrow_mut().notices.push_back(CydWebNotice::new(
                    "wifi-simulated",
                    CydWebNoticeSeverity::Info,
                ));
            }
            CydWebCommand::CalibrationNotNeeded => {
                state.borrow_mut().notices.push_back(CydWebNotice::new(
                    "calibration-not-needed",
                    CydWebNoticeSeverity::Info,
                ));
            }
            CydWebCommand::Reorientate(orientation) => {
                if let Err(error) = orientation_flash_block.save(&orientation) {
                    fatal(&state, format!("orientation save failed: {error:?}"));
                    break;
                }
                state.borrow_mut().orientation = orientation;
            }
        }
    }
    release_control(&state);
    state.borrow_mut().stopped = true;
}

async fn supervise_display<Run, Error>(
    canvas: HtmlCanvasElement,
    config: CydWebAppConfig,
    mut orientation_flash_block: FlashBlockWasm,
    state: Rc<RefCell<SupervisorState>>,
    lifecycle_signal: LifecycleSignal,
    mut inner_main: Run,
    initial_session: Option<(CydDisplayWasm, ButtonWasm)>,
) where
    Run: for<'a> AsyncFnMut(
            &'a mut CydDisplayWasm,
            &'a mut ButtonWasm,
        ) -> Result<CydWebCommand, Error>
        + 'static,
    Error: Debug + 'static,
{
    let mut session = initial_session;
    loop {
        let (mut display, mut button) = match session.take() {
            Some(session) => session,
            None => {
                let orientation = state.borrow().orientation;
                match CydSimulatorWasm::new_display_with_style(
                    canvas.clone(),
                    orientation,
                    config.background,
                    config.foreground,
                    config.font,
                ) {
                    Ok(simulator) => {
                        let (display, button, control) = simulator.into_parts();
                        replace_control(&state, control);
                        (display, button)
                    }
                    Err(error) => {
                        fatal(&state, format!("simulator construction failed: {error:?}"));
                        break;
                    }
                }
            }
        };

        let command = match select(
            inner_main(&mut display, &mut button),
            lifecycle_signal.wait(),
        )
        .await
        {
            Either::First(result) => match result {
                Ok(command) => command,
                Err(error) => {
                    fatal(&state, format!("application failed: {error:?}"));
                    break;
                }
            },
            Either::Second(request) => match apply_display_host_request(
                request,
                &config,
                &mut orientation_flash_block,
                &state,
            ) {
                Ok(()) => CydWebCommand::Restart,
                Err(error) => {
                    fatal(&state, error);
                    break;
                }
            },
        };

        release_control(&state);
        match command {
            CydWebCommand::Stop => break,
            CydWebCommand::Restart => {}
            CydWebCommand::ResetWifi => {
                super::WifiSimulatorWasm::new(config.storage_namespace).reset();
                state.borrow_mut().notices.push_back(CydWebNotice::new(
                    "wifi-simulated",
                    CydWebNoticeSeverity::Info,
                ));
            }
            CydWebCommand::CalibrationNotNeeded => {
                state.borrow_mut().notices.push_back(CydWebNotice::new(
                    "calibration-not-needed",
                    CydWebNoticeSeverity::Info,
                ));
            }
            CydWebCommand::Reorientate(orientation) => {
                if let Err(error) = orientation_flash_block.save(&orientation) {
                    fatal(&state, format!("orientation save failed: {error:?}"));
                    break;
                }
                state.borrow_mut().orientation = orientation;
            }
        }
    }
    release_control(&state);
    state.borrow_mut().stopped = true;
}

fn replace_control(state: &Rc<RefCell<SupervisorState>>, control: CydSimulatorControlWasm) {
    let mut state = state.borrow_mut();
    state.live_control = Some(control);
}

fn release_control(state: &Rc<RefCell<SupervisorState>>) {
    if let Some(control) = state.borrow_mut().live_control.take() {
        control.reset_transient_state();
    }
}

fn fatal(state: &Rc<RefCell<SupervisorState>>, message: String) {
    let mut state = state.borrow_mut();
    state.notices.push_back(CydWebNotice::fatal(message));
}

fn apply_host_request(
    request: HostRequest,
    config: &CydWebAppConfig,
    orientation_flash_block: &mut FlashBlockWasm,
    state: &Rc<RefCell<SupervisorState>>,
) -> Result<(), String> {
    if matches!(request, HostRequest::ClearStorage) {
        orientation_flash_block
            .clear()
            .map_err(|error| format!("storage clear failed: {error:?}"))?;
        state.borrow_mut().orientation = config.initial_orientation;
    }
    Ok(())
}

fn apply_display_host_request(
    request: HostRequest,
    config: &CydWebAppConfig,
    orientation_flash_block: &mut FlashBlockWasm,
    state: &Rc<RefCell<SupervisorState>>,
) -> Result<(), String> {
    if matches!(request, HostRequest::ClearStorage) {
        orientation_flash_block
            .clear()
            .map_err(|error| format!("storage clear failed: {error:?}"))?;
        state.borrow_mut().orientation = config.initial_orientation;
    }
    Ok(())
}
