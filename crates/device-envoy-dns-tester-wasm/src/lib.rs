use std::cell::{Cell, RefCell};

use device_envoy_core::{
    UnwrapInfallible as _,
    button::Button as _,
    cyd::{
        CydDisplay as _, CydTouch as _, CydTouchUncalibrated as _,
        display::{CydFrame, Orientation},
        touch::{
            TouchEvent,
            calibration::{CalibrationConfig, ensure_calibration},
        },
    },
    flash_block::FlashBlock as _,
    wasm::{
        ButtonWasmSource, CydDisplayWasm, CydTouchWasm, CydTouchWasmSource, CydWasm, FlashBlockWasm,
    },
};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb888,
    primitives::Rectangle,
    text::{Baseline, Text},
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const DNS_HOSTNAME: &str = "example.com";
const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white
const STATUS_LINE_HEIGHT: i32 = 20;
const CONTROL_HEIGHT: u32 = 24;

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
    last_latency_millis: u32,
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
            // Calibration is always completed in landscape; the next app boot
            // applies the persisted orientation just like the hardware flow.
            self.orientation.set(saved_orientation);
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
        let Some(display) = &mut state.display else {
            return Ok(());
        };
        let mut frame = Self::render(display, taps, successes, failures, last_latency_millis);
        frame.flush().await.map_err(|error| match error {})
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
        let Some(TouchEvent::Down { point }) = touch.read().unwrap_infallible() else {
            return "idle".into();
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
                Control::Wifi => Self::action("WiFi unavailable in browser"),
            };
        }

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

    fn render(
        display: &mut CydDisplayWasm,
        taps: u32,
        successes: u32,
        failures: u32,
        last_latency_millis: u32,
    ) -> device_envoy_core::wasm::CydFrameWasm<'_> {
        let screen_size = display.screen_size();
        let foreground = display.foreground_565();
        let mut frame = display.full_frame_mut();
        CydFrame::clear(&mut frame);
        let style = MonoTextStyle::new(&FONT_6X10, foreground);
        let lines = [
            "Tap Screen",
            DNS_HOSTNAME,
            &format!("DNS Queries: {taps}"),
            &format!("DNS Successes: {successes}"),
            &format!("DNS Failures: {failures}"),
            &format!("Last latency: {last_latency_millis}ms"),
        ];
        for (line_index, line) in lines.into_iter().enumerate() {
            Text::with_baseline(
                line,
                Point::new(4, line_index as i32 * STATUS_LINE_HEIGHT),
                style,
                Baseline::Top,
            )
            .draw(&mut frame)
            .unwrap_infallible();
        }
        Text::with_baseline(
            "Tap Settings:",
            Point::new(4, screen_size.height as i32 - 2 * STATUS_LINE_HEIGHT),
            style,
            Baseline::Top,
        )
        .draw(&mut frame)
        .unwrap_infallible();
        for (control, label) in [
            (Control::Orientation, "ROT"),
            (Control::Calibration, "CAL"),
            (Control::Wifi, "WiFi"),
        ]
        .into_iter()
        {
            let rectangle = control_rectangle(control, screen_size);
            Text::with_baseline(
                label,
                rectangle.top_left + Point::new(8, 5),
                style,
                Baseline::Top,
            )
            .draw(&mut frame)
            .unwrap_infallible();
        }
        frame
    }
}

#[derive(Clone, Copy)]
enum Control {
    Orientation,
    Calibration,
    Wifi,
}

fn control_rectangle(control: Control, size: Size) -> Rectangle {
    let index = match control {
        Control::Orientation => 0,
        Control::Calibration => 1,
        Control::Wifi => 2,
    };
    Rectangle::new(
        Point::new(
            size.width as i32 * index / 3,
            size.height as i32 - CONTROL_HEIGHT as i32,
        ),
        Size::new(size.width / 3, CONTROL_HEIGHT),
    )
}

fn control_at(point: Point, size: Size) -> Option<Control> {
    [Control::Orientation, Control::Calibration, Control::Wifi]
        .into_iter()
        .find(|control| control_rectangle(*control, size).contains(point))
}

fn map_to_orientation(orientation: Orientation, point: Point) -> Point {
    match orientation {
        Orientation::Landscape => point,
        Orientation::Portrait => Orientation::PortraitInverted.map_landscape_point(point),
        Orientation::LandscapeInverted => Orientation::LandscapeInverted.map_landscape_point(point),
        Orientation::PortraitInverted => Orientation::Portrait.map_landscape_point(point),
    }
}

fn map_to_landscape(orientation: Orientation, point: Point) -> Point {
    match orientation {
        Orientation::Landscape => point,
        Orientation::Portrait => Orientation::PortraitInverted.map_landscape_point(point),
        Orientation::LandscapeInverted => Orientation::LandscapeInverted.map_landscape_point(point),
        Orientation::PortraitInverted => Orientation::Portrait.map_landscape_point(point),
    }
}
