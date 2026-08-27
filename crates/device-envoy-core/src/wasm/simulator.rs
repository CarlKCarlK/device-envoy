//! Shared browser-facing construction and input control for a simulated CYD.
//!
//! See the shared [`crate::cyd`] API and the compiled [`crate::wasm`] example
//! for the device-level drawing path. This module adds the browser shell's
//! canvas construction and interactive touch/BOOT controls:
//!
//! ```rust,no_run
//! use device_envoy_core::{
//!     UnwrapInfallible,
//!     button::Button,
//!     cyd::{CydDisplay, CydTouch, display::Orientation, touch::TouchEvent},
//! };
//! use device_envoy_core::wasm::simulator::{
//!     CydSimulatorControlWasm, CydSimulatorWasm,
//! };
//! use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
//! use web_sys::HtmlCanvasElement;
//! use wasm_bindgen::JsValue;
//!
//! fn start_simulation(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
//!     let standard = CydSimulatorWasm::new(canvas.clone(), Orientation::Landscape)?;
//!     drop(standard);
//!     let simulator = CydSimulatorWasm::new_with_style(
//!         canvas,
//!         Orientation::Landscape,
//!         Rgb888::BLACK,
//!         Rgb888::WHITE,
//!         &embedded_graphics::mono_font::ascii::FONT_6X10,
//!     )?;
//!     let (cyd, button, control): (_, _, CydSimulatorControlWasm) = simulator.into_parts();
//!     assert_eq!(cyd.display().screen_size(), Orientation::Landscape.size());
//!     assert_eq!(control.orientation(), Orientation::Landscape);
//!     assert!(!control.orientation_is_inverted());
//!     let (_, mut touch) = cyd.owned_parts();
//!     control.touch_down(10.0, 20.0);
//!     assert!(matches!(
//!         touch.read().unwrap_infallible(),
//!         Some(TouchEvent::Down { .. }),
//!     ));
//!     control.touch_move(12.0, 22.0);
//!     assert!(matches!(
//!         touch.read().unwrap_infallible(),
//!         Some(TouchEvent::Move { .. }),
//!     ));
//!     control.touch_up();
//!     assert!(matches!(touch.read().unwrap_infallible(), Some(TouchEvent::Up)));
//!     control.boot_down();
//!     assert!(button.is_pressed());
//!     control.boot_up();
//!     assert!(!button.is_pressed());
//!     control.reset_transient_state();
//!     Ok(())
//! }
//! ```

use embedded_graphics::{
    mono_font::{MonoFont, ascii::FONT_6X10},
    pixelcolor::Rgb888,
};
use std::{cell::RefCell, thread_local, vec::Vec};
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use super::{ButtonWasm, ButtonWasmSource, CydTouchWasmSource, CydWasm, next_animation_frame};
use crate::button::Button;
use crate::cyd::display::Orientation;
use crate::wifi_auto::WifiAutoEvent;

const WIFI_CAPTIVE_PORTAL_WAIT_FRAMES: usize = 15;
const WIFI_CONNECT_WAIT_FRAMES: usize = 90;

const BACKGROUND_COLOR: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND_COLOR: Rgb888 = Rgb888::new(230, 230, 230); // near-white

/// The reusable resources and input protocol for one browser CYD instance.
/// See the compiled [`crate::wasm::simulator`] example.
pub struct CydSimulatorWasm {
    cyd: CydWasm,
    button_source: ButtonWasmSource,
    control: CydSimulatorControlWasm,
}

/// Browser input and lifecycle control shared by an application launcher.
/// See the compiled [`crate::wasm::simulator`] example.
#[wasm_bindgen]
#[derive(Clone)]
pub struct CydSimulatorControlWasm {
    touch_source: Option<CydTouchWasmSource>,
    button_source: ButtonWasmSource,
    orientation: Orientation,
}

/// Result of the shared browser Wi-Fi connection simulation.
/// See the compiled [`WifiSimulatorWasm`] example.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WifiConnectOutcome {
    /// The simulated client connection completed.
    /// See the compiled [`WifiSimulatorWasm`] example.
    Connected,
    /// BOOT interrupted the connection and requests a reset.
    /// See the compiled [`WifiSimulatorWasm`] example.
    ResetRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WifiSimulatorPhase {
    Disconnected,
    CaptivePortal,
    Connecting,
    Connected,
}

thread_local! {
    static WIFI_SIMULATOR_PHASES: RefCell<Vec<(&'static str, WifiSimulatorPhase)>> =
        const { RefCell::new(Vec::new()) };
}

/// A deterministic browser substitute for the platform Wi-Fi auto-provisioner.
/// See the [`crate::wasm`] module for the browser implementation overview.
///
/// ```rust,no_run
/// use core::convert::Infallible;
/// use device_envoy_core::wasm::{
///     ButtonWasmSource, WifiConnectOutcome, WifiSimulatorWasm,
/// };
///
/// async fn connect() -> Result<(), Infallible> {
///     let outcomes = [
///         WifiConnectOutcome::Connected,
///         WifiConnectOutcome::ResetRequested,
///     ];
///     assert_eq!(outcomes.len(), 2);
///     let button_source = ButtonWasmSource::new();
///     let mut button = button_source.button();
///     let wifi_simulator = WifiSimulatorWasm::new("counter-demo");
///     wifi_simulator.reset();
///     let outcome = wifi_simulator
///         .connect(&mut button, async |_event| Ok::<(), Infallible>(()))
///         .await?;
///     assert_eq!(outcome, WifiConnectOutcome::Connected);
///     Ok(())
/// }
/// ```
pub struct WifiSimulatorWasm {
    storage_namespace: &'static str,
}

impl WifiSimulatorWasm {
    /// Construct a simulated Wi-Fi resource scoped to an application namespace.
    /// See the compiled [`WifiSimulatorWasm`] example.
    #[must_use]
    pub const fn new(storage_namespace: &'static str) -> Self {
        Self { storage_namespace }
    }

    /// Reset this application's simulated Wi-Fi resource to its disconnected state.
    /// See the compiled [`WifiSimulatorWasm`] example.
    pub fn reset(&self) {
        set_phase(self.storage_namespace, WifiSimulatorPhase::Disconnected);
    }

    fn phase(&self) -> WifiSimulatorPhase {
        WIFI_SIMULATOR_PHASES.with(|phases| {
            phases
                .borrow()
                .iter()
                .find(|(namespace, _)| *namespace == self.storage_namespace)
                .map_or(WifiSimulatorPhase::Disconnected, |(_, phase)| *phase)
        })
    }

    /// Run the deterministic browser connection sequence.
    /// See the compiled [`WifiSimulatorWasm`] example.
    pub async fn connect<OnEvent, Error>(
        &self,
        button: &mut ButtonWasm,
        mut on_event: OnEvent,
    ) -> Result<WifiConnectOutcome, Error>
    where
        OnEvent: AsyncFnMut(WifiAutoEvent) -> Result<(), Error>,
    {
        if self.phase() == WifiSimulatorPhase::Connected {
            return Ok(WifiConnectOutcome::Connected);
        }

        set_phase(self.storage_namespace, WifiSimulatorPhase::CaptivePortal);
        on_event(WifiAutoEvent::CaptivePortalReady).await?;
        if wait_for_wifi_frames(button, WIFI_CAPTIVE_PORTAL_WAIT_FRAMES).await {
            return Ok(WifiConnectOutcome::ResetRequested);
        }

        set_phase(self.storage_namespace, WifiSimulatorPhase::Connecting);
        on_event(WifiAutoEvent::Connecting {
            try_index: 0,
            try_count: 1,
        })
        .await?;
        if wait_for_wifi_frames(button, WIFI_CONNECT_WAIT_FRAMES).await {
            return Ok(WifiConnectOutcome::ResetRequested);
        }

        set_phase(self.storage_namespace, WifiSimulatorPhase::Connected);
        Ok(WifiConnectOutcome::Connected)
    }
}

fn set_phase(storage_namespace: &'static str, phase: WifiSimulatorPhase) {
    WIFI_SIMULATOR_PHASES.with(|phases| {
        let mut phases = phases.borrow_mut();
        if let Some((_, current_phase)) = phases
            .iter_mut()
            .find(|(namespace, _)| *namespace == storage_namespace)
        {
            *current_phase = phase;
        } else {
            phases.push((storage_namespace, phase));
        }
    });
}

impl CydSimulatorWasm {
    /// Construct a simulated CYD using the standard CYD browser palette.
    /// See the compiled [`crate::wasm::simulator`] example.
    pub fn new(canvas: HtmlCanvasElement, orientation: Orientation) -> Result<Self, JsValue> {
        Self::new_with_style(
            canvas,
            orientation,
            BACKGROUND_COLOR,
            FOREGROUND_COLOR,
            &FONT_6X10,
        )
    }

    /// Construct a simulated CYD with an application-specific display style.
    /// See the compiled [`crate::wasm::simulator`] example.
    pub fn new_with_style(
        canvas: HtmlCanvasElement,
        orientation: Orientation,
        background_color: Rgb888,
        foreground_color: Rgb888,
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
            background_color,
            foreground_color,
            font,
            touch_source.clone(),
        );
        let control = CydSimulatorControlWasm {
            touch_source: Some(touch_source),
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
    /// See the compiled [`crate::wasm::simulator`] example.
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
    /// See the compiled [`crate::wasm::simulator`] example.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.orientation
    }
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
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = orientation_is_inverted)]
    pub fn orientation_is_inverted(&self) -> bool {
        matches!(
            self.orientation,
            Orientation::LandscapeInverted | Orientation::PortraitInverted
        )
    }

    /// Forward a browser pointer-down position in logical canvas coordinates.
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = touch_down)]
    pub fn touch_down(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation, x, y);
        if let Some(touch_source) = &self.touch_source {
            touch_source.touch_down(point.0, point.1);
        }
    }

    /// Forward a browser pointer-move position in logical canvas coordinates.
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = touch_move)]
    pub fn touch_move(&self, x: f32, y: f32) {
        let point = map_to_landscape(self.orientation, x, y);
        if let Some(touch_source) = &self.touch_source {
            touch_source.touch_move(point.0, point.1);
        }
    }

    /// Forward a browser pointer-up or pointer-cancel event.
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = touch_up)]
    pub fn touch_up(&self) {
        if let Some(touch_source) = &self.touch_source {
            touch_source.touch_up();
        }
    }

    /// Forward a physical BOOT-button press.
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = boot_down)]
    pub fn boot_down(&self) {
        self.button_source.press();
    }

    /// Forward a physical BOOT-button release.
    /// See the compiled [`crate::wasm::simulator`] example.
    #[wasm_bindgen(js_name = boot_up)]
    pub fn boot_up(&self) {
        self.button_source.release();
    }

    /// Clear transient browser input after a simulated reset.
    /// See the compiled [`crate::wasm::simulator`] example.
    pub fn reset_transient_state(&self) {
        if let Some(touch_source) = &self.touch_source {
            touch_source.touch_up();
        }
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

    #[test]
    fn wifi_state_is_scoped_by_storage_namespace() {
        set_phase("app-a", WifiSimulatorPhase::Connected);
        assert_eq!(
            WifiSimulatorWasm::new("app-a").phase(),
            WifiSimulatorPhase::Connected
        );
        assert_eq!(
            WifiSimulatorWasm::new("app-b").phase(),
            WifiSimulatorPhase::Disconnected
        );

        WifiSimulatorWasm::new("app-a").reset();
        assert_eq!(
            WifiSimulatorWasm::new("app-a").phase(),
            WifiSimulatorPhase::Disconnected
        );
        assert_eq!(
            WifiSimulatorWasm::new("app-b").phase(),
            WifiSimulatorPhase::Disconnected
        );
    }
}
