//! Shared browser-facing construction and input control for a simulated CYD.

use embedded_graphics::{
    mono_font::{MonoFont, ascii::FONT_6X10},
    pixelcolor::Rgb888,
};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::{ButtonWasm, ButtonWasmSource, CydTouchWasmSource, CydWasm, next_animation_frame};
use crate::button::Button;
use crate::cyd::display::Orientation;

const WIFI_CAPTIVE_PORTAL_WAIT_FRAMES: usize = 15;
const WIFI_CONNECT_WAIT_FRAMES: usize = 90;

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

/// Severity assigned to a browser simulator notice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulatorNoticeSeverity {
    /// A recoverable informational message.
    Info,
    /// A recoverable warning that needs user attention.
    Warning,
    /// An unrecoverable application error.
    Fatal,
}

/// A typed request for the shared browser notice facility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulatorNoticeRequest {
    /// Stable identifier interpreted by the application shell.
    pub id: &'static str,
    /// Severity controls accessibility presentation and loop behavior.
    pub severity: SimulatorNoticeSeverity,
}

impl SimulatorNoticeRequest {
    /// Request the recoverable simulated Wi-Fi setup notice.
    pub const fn wifi_setup() -> Self {
        Self {
            id: "wifi-setup",
            severity: SimulatorNoticeSeverity::Warning,
        }
    }

    /// Request the recoverable simulated Wi-Fi connecting notice.
    pub const fn wifi_connecting() -> Self {
        Self {
            id: "wifi-connecting",
            severity: SimulatorNoticeSeverity::Info,
        }
    }

    /// Request the recoverable simulated Wi-Fi unavailable notice.
    pub const fn wifi_unavailable() -> Self {
        Self {
            id: "wifi-unavailable",
            severity: SimulatorNoticeSeverity::Warning,
        }
    }

    /// Request an unrecoverable simulator runtime notice.
    pub const fn runtime_error() -> Self {
        Self {
            id: "runtime-error",
            severity: SimulatorNoticeSeverity::Fatal,
        }
    }

    /// Return whether this request must stop the application loop.
    #[must_use]
    pub const fn is_fatal(self) -> bool {
        matches!(self.severity, SimulatorNoticeSeverity::Fatal)
    }
}

/// Result of submitting a notice to the application-facing simulator queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulatorNoticeDisposition {
    /// Keep the application input loop running.
    Continue,
    /// Stop the application input loop after reporting the notice.
    Terminate,
}

/// Classify a notice request according to its typed severity.
#[must_use]
pub const fn simulator_notice_disposition(
    request: SimulatorNoticeRequest,
) -> SimulatorNoticeDisposition {
    if request.is_fatal() {
        SimulatorNoticeDisposition::Terminate
    } else {
        SimulatorNoticeDisposition::Continue
    }
}

/// Events emitted by the shared browser Wi-Fi connection simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiConnectEvent {
    /// The simulated captive-portal setup phase is ready.
    CaptivePortalReady,
    /// The simulated client connection has begun.
    Connecting { try_index: u8, try_count: u8 },
    /// The simulated connection failed.
    ConnectionFailed,
}

/// Result of the shared browser Wi-Fi connection simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiConnectOutcome {
    /// The simulated client connection completed.
    Connected,
    /// BOOT interrupted the connection and requests a reset.
    ResetRequested,
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

/// Simulate the Wi-Fi connection phase used by applications with real Wi-Fi.
pub async fn simulate_wifi_connect<OnEvent, Error>(
    button: &mut ButtonWasm,
    mut on_event: OnEvent,
) -> Result<WifiConnectOutcome, Error>
where
    OnEvent: AsyncFnMut(WifiConnectEvent) -> Result<(), Error>,
{
    on_event(WifiConnectEvent::CaptivePortalReady).await?;
    if wait_for_wifi_frames(button, WIFI_CAPTIVE_PORTAL_WAIT_FRAMES).await {
        return Ok(WifiConnectOutcome::ResetRequested);
    }

    on_event(WifiConnectEvent::Connecting {
        try_index: 0,
        try_count: 1,
    })
    .await?;
    if wait_for_wifi_frames(button, WIFI_CONNECT_WAIT_FRAMES).await {
        return Ok(WifiConnectOutcome::ResetRequested);
    }

    Ok(WifiConnectOutcome::Connected)
}

async fn wait_for_wifi_frames(button: &ButtonWasm, frame_count: usize) -> bool {
    for _ in 0..frame_count {
        if button.is_pressed() {
            return true;
        }
        next_animation_frame().await;
    }
    false
}

#[wasm_bindgen]
impl CydSimulatorControlWasm {
    /// Return whether the simulated display is presented upside down.
    #[wasm_bindgen(js_name = orientation_is_inverted)]
    pub fn orientation_is_inverted(&self) -> bool {
        matches!(
            self.orientation,
            Orientation::LandscapeInverted | Orientation::PortraitInverted
        )
    }

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
    fn recoverable_notices_continue_the_application_loop() {
        assert_eq!(
            simulator_notice_disposition(SimulatorNoticeRequest::wifi_setup()),
            SimulatorNoticeDisposition::Continue
        );
        assert_eq!(
            simulator_notice_disposition(SimulatorNoticeRequest::wifi_connecting()),
            SimulatorNoticeDisposition::Continue
        );
    }

    #[test]
    fn fatal_notices_terminate_the_application_loop() {
        assert_eq!(
            simulator_notice_disposition(SimulatorNoticeRequest::runtime_error()),
            SimulatorNoticeDisposition::Terminate
        );
    }

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
