//! A device abstraction for complete CYD browser applications.
//!
//! [`start`] creates the stable browser supervisor and returns [`Handle`]. The
//! supervisor constructs fresh [`Capabilities`] for each run, allowing the
//! application to select focused capabilities for unchanged generic core code.
//! [`Command`] communicates application policy back to the supervisor.
//!
//! ## Compiled browser-shell example
//!
//! The HTML page supplies a `<canvas id="cyd-canvas">`; Rust supplies the
//! presentation metadata and an async application function. The returned
//! [`Handle`] is the stable JavaScript-facing control surface.
//!
//! ```rust,no_run
//! use core::convert::Infallible;
//! use device_envoy_core::{
//!     button::Button,
//!     cyd::{CydDisplay, display::Orientation},
//!     dns::Dns,
//!     wasm::cyd_web::{
//!         self, Capabilities, Command, Config, Handle, Notice, NoticeSeverity, PageInfo,
//!     },
//! };
//! use embedded_graphics::{
//!     mono_font::ascii::FONT_6X10,
//!     pixelcolor::{Rgb888, RgbColor},
//! };
//! use wasm_bindgen::JsValue;
//!
//! const CONFIG: Config = Config::new(
//!     "counter-demo",
//!     Orientation::Landscape,
//!     Rgb888::BLACK,
//!     Rgb888::WHITE,
//!     &FONT_6X10,
//! );
//! const PAGE: PageInfo = PageInfo::new(
//!     "Counter",
//!     "A touch-controlled counter",
//!     "The same application logic used by the hardware builds.",
//!     "Touch the display; BOOT resets.",
//!     "https://example.invalid/counter.rs",
//! );
//!
//! async fn inner_main(
//!     mut capabilities: Capabilities,
//! ) -> Result<Command, Infallible> {
//!     assert_eq!(capabilities.cyd.display().screen_size().width, 320);
//!     assert!(!capabilities.button.is_pressed());
//!     capabilities.clock_sync.show();
//!     capabilities.wifi_simulator.reset();
//!     let addresses = capabilities.dns_simulator.resolve("example.com").await?;
//!     assert!(!addresses.is_empty());
//!     Ok(Command::Stop)
//! }
//!
//! fn launch() -> Result<(), JsValue> {
//!     assert_eq!(CONFIG.storage_namespace, "counter-demo");
//!     assert_eq!(CONFIG.initial_orientation, Orientation::Landscape);
//!     assert_eq!(CONFIG.background_color, Rgb888::BLACK);
//!     assert_eq!(CONFIG.foreground_color, Rgb888::WHITE);
//!     assert_eq!(CONFIG.font.character_size.width, FONT_6X10.character_size.width);
//!     assert_eq!(PAGE.title, "Counter");
//!     assert!(!PAGE.preview.is_empty());
//!     assert!(!PAGE.description.is_empty());
//!     assert!(!PAGE.controls.is_empty());
//!     assert!(PAGE.core_code_url.ends_with("counter.rs"));
//!
//!     let handle: Handle = cyd_web::start("cyd-canvas", CONFIG, PAGE, inner_main)?;
//!     handle.touch_down(20.0, 30.0);
//!     handle.touch_move(22.0, 32.0);
//!     handle.touch_up();
//!     handle.boot_down();
//!     handle.boot_up();
//!     assert!(!handle.orientation_is_inverted());
//!     assert_eq!(handle.page_title(), "Counter");
//!     assert_eq!(handle.page_preview(), PAGE.preview);
//!     assert_eq!(handle.page_description(), PAGE.description);
//!     assert_eq!(handle.page_controls(), PAGE.controls);
//!     assert_eq!(handle.page_core_code_url(), PAGE.core_code_url);
//!     handle.set_clock_time_of_day(12 * 60 * 60)?;
//!     handle.use_live_clock();
//!     handle.clock_control_is_visible();
//!     if let Some(notice) = handle.take_notice() {
//!         inspect_notice(notice);
//!     }
//!     handle.request_restart();
//!     handle.clear_storage_and_restart();
//!     Ok(())
//! }
//!
//! fn inspect_notice(notice: Notice) {
//!     assert!(!notice.id().is_empty());
//!     notice.severity();
//!     notice.detail();
//! }
//!
//! const NOTICE_SEVERITIES: [NoticeSeverity; 3] = [
//!     NoticeSeverity::Info,
//!     NoticeSeverity::Warning,
//!     NoticeSeverity::Fatal,
//! ];
//!
//! fn every_command(index: u8) -> Command {
//!     match index {
//!         0 => Command::Restart,
//!         1 => Command::CalibrationNotNeeded,
//!         2 => Command::ResetWifi,
//!         3 => Command::Reorientate(Orientation::Portrait),
//!         _ => Command::Stop,
//!     }
//! }
//! ```

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
};

use embassy_futures::select::{Either, select};
use embedded_graphics::{mono_font::MonoFont, pixelcolor::Rgb888};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{HtmlCanvasElement, window};

use super::{
    ButtonWasm, ClockSyncWasm, CydSimulatorControlWasm, CydSimulatorWasm, CydWasm,
    DnsSimulatorWasm, FlashBlockWasm, WifiSimulatorWasm,
};
use crate::cyd::display::Orientation;
use crate::flash_block::FlashBlock as _;

#[derive(Clone, Copy)]
/// Presentation and persistent-storage settings for a [`Capabilities`]
/// session.
/// The compiled browser-shell example on [`crate::wasm::cyd_web`] constructs
/// and reads every field.
pub struct Config {
    /// Namespace used for orientation and simulated Wi-Fi state.
    pub storage_namespace: &'static str,
    /// Orientation used when no saved orientation exists.
    pub initial_orientation: Orientation,
    /// Canvas background color.
    pub background_color: Rgb888,
    /// Canvas foreground color.
    pub foreground_color: Rgb888,
    /// Font used by the simulated display.
    pub font: &'static MonoFont<'static>,
}

impl Config {
    /// Construct presentation settings for [`start`].
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub const fn new(
        storage_namespace: &'static str,
        initial_orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self {
        Self {
            storage_namespace,
            initial_orientation,
            background_color,
            foreground_color,
            font,
        }
    }
}

#[derive(Clone, Copy)]
/// Browser-facing metadata displayed by the shared CYD simulator shell.
/// The compiled browser-shell example on [`crate::wasm::cyd_web`] constructs
/// and reads every field.
pub struct PageInfo {
    /// Page title.
    pub title: &'static str,
    /// Short preview text.
    pub preview: &'static str,
    /// Longer application description.
    pub description: &'static str,
    /// Human-readable interaction instructions.
    pub controls: &'static str,
    /// Link to the platform-neutral application source.
    pub core_code_url: &'static str,
}

impl PageInfo {
    /// Construct page metadata for [`start`].
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub const fn new(
        title: &'static str,
        preview: &'static str,
        description: &'static str,
        controls: &'static str,
        core_code_url: &'static str,
    ) -> Self {
        Self {
            title,
            preview,
            description,
            controls,
            core_code_url,
        }
    }
}

/// Complete capability container supplied to each application run.
///
/// A launcher receives this value from [`start`], selects focused capabilities,
/// and returns a [`Command`].
/// The compiled browser-shell example on [`crate::wasm::cyd_web`] reads every
/// field.
///
/// ```rust,no_run
/// # use core::convert::Infallible;
/// # use device_envoy_core::{cyd::CydDisplay, wasm::cyd_web};
/// #
/// async fn inner_main(
///     mut capabilities: cyd_web::Capabilities,
/// ) -> Result<cyd_web::Command, Infallible> {
///     capabilities.clock_sync.show();
///     assert_eq!(capabilities.cyd.display().screen_size().width, 320);
///     Ok(cyd_web::Command::Stop)
/// }
/// #
/// # fn receives_capabilities(_capabilities: cyd_web::Capabilities) {}
/// ```
pub struct Capabilities {
    /// CYD display and touch capability.
    pub cyd: CydWasm,
    /// BOOT-button capability.
    pub button: ButtonWasm,
    /// Browser-backed clock capability.
    pub clock_sync: ClockSyncWasm,
    /// Simulated Wi-Fi capability.
    pub wifi_simulator: WifiSimulatorWasm,
    /// Deterministic simulated DNS capability.
    pub dns_simulator: DnsSimulatorWasm,
}

/// Result requested by an application after one run.
/// The compiled browser-shell example on [`crate::wasm::cyd_web`] constructs
/// every variant.
pub enum Command {
    /// Restart the current session.
    Restart,
    /// Report that physical calibration is unnecessary in the browser.
    CalibrationNotNeeded,
    /// Clear simulated Wi-Fi state and restart.
    ResetWifi,
    /// Persist and apply a new display orientation.
    Reorientate(Orientation),
    /// Stop the supervisor.
    Stop,
}

#[wasm_bindgen(js_name = CydWebNoticeSeverity)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Severity assigned to a framework notice.
///
/// The compiled browser-shell example on [`crate::wasm::cyd_web`] constructs
/// every variant.
pub enum NoticeSeverity {
    /// Informational notice.
    Info,
    /// Recoverable warning.
    Warning,
    /// Terminal runtime failure.
    Fatal,
}

#[wasm_bindgen(js_name = CydWebNotice)]
#[derive(Clone, Debug)]
/// Typed notice emitted by the framework for the shared browser shell.
/// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
pub struct Notice {
    id: String,
    severity: NoticeSeverity,
    detail: Option<String>,
}

impl Notice {
    fn new(id: impl Into<String>, severity: NoticeSeverity) -> Self {
        Self {
            id: id.into(),
            severity,
            detail: None,
        }
    }
    fn fatal(detail: String) -> Self {
        Self {
            id: "runtime-error".into(),
            severity: NoticeSeverity::Fatal,
            detail: Some(detail),
        }
    }
}

#[wasm_bindgen(js_class = CydWebNotice)]
impl Notice {
    /// Return the stable notice identifier.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn id(&self) -> String {
        self.id.clone()
    }
    /// Return the notice severity.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn severity(&self) -> NoticeSeverity {
        self.severity
    }
    /// Return optional diagnostic detail.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
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
            Poll::Ready(request)
        } else {
            state.waker = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

struct SupervisorState {
    live_control: Option<CydSimulatorControlWasm>,
    notices: std::collections::VecDeque<Notice>,
    orientation: Orientation,
    stopped: bool,
    page_info: PageInfo,
    clock_time_of_day: Rc<Cell<Option<u32>>>,
    clock_control_visible: Rc<Cell<bool>>,
}

#[wasm_bindgen(js_name = CydWebAppHandle)]
/// Stable browser control handle returned by [`start`].
/// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
pub struct Handle {
    state: Rc<RefCell<SupervisorState>>,
    lifecycle_signal: LifecycleSignal,
}

impl Handle {
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

#[wasm_bindgen(js_class = CydWebAppHandle)]
impl Handle {
    /// Press the simulated touch panel at canvas coordinates.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn touch_down(&self, position_x: f32, position_y: f32) {
        self.with_control(|control| control.touch_down(position_x, position_y));
    }
    /// Move the simulated touch point.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn touch_move(&self, position_x: f32, position_y: f32) {
        self.with_control(|control| control.touch_move(position_x, position_y));
    }
    /// Release the simulated touch panel.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn touch_up(&self) {
        self.with_control(CydSimulatorControlWasm::touch_up);
    }
    /// Press the simulated BOOT button.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn boot_down(&self) {
        self.with_control(CydSimulatorControlWasm::boot_down);
    }
    /// Release the simulated BOOT button.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn boot_up(&self) {
        self.with_control(CydSimulatorControlWasm::boot_up);
    }
    /// Return whether the current orientation is inverted.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn orientation_is_inverted(&self) -> bool {
        self.state
            .borrow()
            .live_control
            .as_ref()
            .is_some_and(CydSimulatorControlWasm::orientation_is_inverted)
    }
    /// Remove and return the oldest pending framework notice.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn take_notice(&self) -> Option<Notice> {
        self.state.borrow_mut().notices.pop_front()
    }
    /// Request an application restart.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn request_restart(&self) {
        self.lifecycle_signal.request(HostRequest::Restart);
    }
    /// Clear framework storage and restart the application.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn clear_storage_and_restart(&self) {
        self.lifecycle_signal.request(HostRequest::ClearStorage);
    }
    /// Return whether the application has requested the clock control.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn clock_control_is_visible(&self) -> bool {
        self.state.borrow().clock_control_visible.get()
    }
    /// Set the simulated local time, in seconds after midnight.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn set_clock_time_of_day(&self, seconds_of_day: u32) -> Result<(), JsValue> {
        if seconds_of_day >= 86_400 {
            return Err(JsValue::from_str(
                "time of day must be between 0 and 86399 seconds",
            ));
        }
        self.state
            .borrow()
            .clock_time_of_day
            .set(Some(seconds_of_day));
        Ok(())
    }
    /// Restore the browser's live local clock.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn use_live_clock(&self) {
        self.state.borrow().clock_time_of_day.set(None);
    }
    /// Return the configured page title.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn page_title(&self) -> String {
        self.state.borrow().page_info.title.into()
    }
    /// Return the configured preview text.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn page_preview(&self) -> String {
        self.state.borrow().page_info.preview.into()
    }
    /// Return the configured page description.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn page_description(&self) -> String {
        self.state.borrow().page_info.description.into()
    }
    /// Return the configured interaction instructions.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn page_controls(&self) -> String {
        self.state.borrow().page_info.controls.into()
    }
    /// Return the configured platform-neutral source URL.
    /// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
    pub fn page_core_code_url(&self) -> String {
        self.state.borrow().page_info.core_code_url.into()
    }
}

/// Start a browser CYD application in the shared simulator shell.
/// See the compiled browser-shell example on [`crate::wasm::cyd_web`].
pub fn start<Run, Error>(
    canvas_id: &str,
    config: Config,
    page_info: PageInfo,
    inner_main: Run,
) -> Result<Handle, JsValue>
where
    Run: AsyncFnMut(Capabilities) -> Result<Command, Error> + 'static,
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
        config.background_color,
        config.foreground_color,
        config.font,
    )?;
    let (cyd, button, control) = simulator.into_parts();
    let state = Rc::new(RefCell::new(SupervisorState {
        live_control: Some(control),
        notices: std::collections::VecDeque::new(),
        orientation,
        stopped: false,
        page_info,
        clock_time_of_day: Rc::new(Cell::new(None)),
        clock_control_visible: Rc::new(Cell::new(false)),
    }));
    let lifecycle_signal = LifecycleSignal::new();
    let handle = Handle::new(state.clone(), lifecycle_signal.clone());
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
    config: Config,
    mut orientation_flash_block: FlashBlockWasm,
    state: Rc<RefCell<SupervisorState>>,
    lifecycle_signal: LifecycleSignal,
    mut inner_main: Run,
    initial_session: Option<(CydWasm, ButtonWasm)>,
) where
    Run: AsyncFnMut(Capabilities) -> Result<Command, Error> + 'static,
    Error: Debug + 'static,
{
    let mut session = initial_session;
    loop {
        let (cyd, button) = match session.take() {
            Some(session) => session,
            None => {
                let orientation = state.borrow().orientation;
                match CydSimulatorWasm::new_with_style(
                    canvas.clone(),
                    orientation,
                    config.background_color,
                    config.foreground_color,
                    config.font,
                ) {
                    Ok(simulator) => {
                        let (cyd, button, control) = simulator.into_parts();
                        state.borrow_mut().live_control = Some(control);
                        (cyd, button)
                    }
                    Err(error) => {
                        fatal(&state, format!("simulator construction failed: {error:?}"));
                        break;
                    }
                }
            }
        };
        let (clock_time_of_day, clock_control_visible) = {
            let state_ref = state.borrow();
            (
                state_ref.clock_time_of_day.clone(),
                state_ref.clock_control_visible.clone(),
            )
        };
        let clock_sync =
            ClockSyncWasm::new_with_control_state(clock_time_of_day, clock_control_visible);
        let application = Capabilities {
            cyd,
            button,
            clock_sync,
            wifi_simulator: WifiSimulatorWasm::new(config.storage_namespace),
            dns_simulator: DnsSimulatorWasm::standard(),
        };
        let command = match select(inner_main(application), lifecycle_signal.wait()).await {
            Either::First(result) => match result {
                Ok(command) => command,
                Err(error) => {
                    fatal(&state, format!("application failed: {error:?}"));
                    break;
                }
            },
            Either::Second(request) => {
                match apply_host_request(request, &config, &mut orientation_flash_block, &state) {
                    Ok(()) => Command::Restart,
                    Err(error) => {
                        fatal(&state, error);
                        break;
                    }
                }
            }
        };
        release_control(&state);
        match command {
            Command::Stop => break,
            Command::Restart => {}
            Command::ResetWifi => {
                WifiSimulatorWasm::new(config.storage_namespace).reset();
                state
                    .borrow_mut()
                    .notices
                    .push_back(Notice::new("wifi-simulated", NoticeSeverity::Info));
            }
            Command::CalibrationNotNeeded => state
                .borrow_mut()
                .notices
                .push_back(Notice::new("calibration-not-needed", NoticeSeverity::Info)),
            Command::Reorientate(orientation) => {
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

fn release_control(state: &Rc<RefCell<SupervisorState>>) {
    let control = state.borrow_mut().live_control.take();
    if let Some(control) = control {
        control.reset_transient_state();
    }
}
fn fatal(state: &Rc<RefCell<SupervisorState>>, message: String) {
    state.borrow_mut().notices.push_back(Notice::fatal(message));
}
fn apply_host_request(
    request: HostRequest,
    config: &Config,
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
