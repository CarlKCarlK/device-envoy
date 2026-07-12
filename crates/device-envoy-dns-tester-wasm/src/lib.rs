use std::cell::{Cell, RefCell};

use device_envoy_core::{
    UnwrapInfallible as _,
    button::Button as _,
    cyd::{
        CydDisplay as _, CydTouch as _, CydTouchUncalibrated as _,
        display::{CydFrame, DrawItem, Image565Fixed, Orientation},
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
const CONTROL_HEIGHT: u32 = 24;

const LANDSCAPE_BACKGROUND: Image565Fixed<320, 240, { 320 * 240 }> =
    device_envoy_core::cyd::display::tga!(
        "../../../docs/dns-tester/v1/assets/dns_tester_background_landscape.tga",
        320,
        240
    )
    .to_565();
const PORTRAIT_BACKGROUND: Image565Fixed<240, 320, { 240 * 320 }> =
    device_envoy_core::cyd::display::tga!(
        "../../../docs/dns-tester/v1/assets/dns_tester_background_portrait.tga",
        240,
        320
    )
    .to_565();

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
        let screen_size = display.screen_size();
        let background = background_for_size(screen_size);
        let background_color = display.background_565();
        display
            .draw_items::<1>(
                Rectangle::new(Point::zero(), screen_size),
                background_color,
                [DrawItem::Bitmap {
                    view: background.view(),
                    top_left: Point::zero(),
                }],
            )
            .unwrap_infallible();

        let status = if taps == 0 { "TAP TO TEST" } else { "READY" };
        let latency = if taps == 0 {
            "—".to_owned()
        } else {
            format!("{last_latency_millis} ms")
        };
        let queries = taps.to_string();
        let success_count = successes.to_string();
        let failure_count = failures.to_string();

        if screen_size.width > screen_size.height {
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(220, 0), Size::new(100, 20)),
                status.to_owned(),
                Rgb888::new(127, 220, 255), // bright cyan
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(80, 78), Size::new(160, 28)),
                latency,
                Rgb888::new(185, 210, 223), // pale blue-white
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(70, 164), Size::new(38, 18)),
                queries,
                Rgb888::new(185, 210, 223), // pale blue-white
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(177, 164), Size::new(38, 18)),
                success_count,
                Rgb888::new(130, 220, 150), // soft green
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(282, 164), Size::new(38, 18)),
                failure_count,
                Rgb888::new(240, 125, 115), // coral red
            )
            .await?;
        } else {
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(150, 0), Size::new(90, 20)),
                status.to_owned(),
                Rgb888::new(127, 220, 255), // bright cyan
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(50, 98), Size::new(140, 25)),
                latency,
                Rgb888::new(185, 210, 223), // pale blue-white
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(195, 168), Size::new(45, 16)),
                queries,
                Rgb888::new(185, 210, 223), // pale blue-white
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(195, 185), Size::new(45, 16)),
                success_count,
                Rgb888::new(130, 220, 150), // soft green
            )
            .await?;
            draw_text_value(
                display,
                background,
                Rectangle::new(Point::new(195, 202), Size::new(45, 16)),
                failure_count,
                Rgb888::new(240, 125, 115), // coral red
            )
            .await?;
        }
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
}

#[derive(Clone, Copy)]
enum Background {
    Landscape,
    Portrait,
}

impl Background {
    fn view(self) -> device_envoy_core::cyd::display::Image565View {
        match self {
            Self::Landscape => LANDSCAPE_BACKGROUND.view(),
            Self::Portrait => PORTRAIT_BACKGROUND.view(),
        }
    }

    fn view_rect(self, rectangle: Rectangle) -> device_envoy_core::cyd::display::Image565View {
        match self {
            Self::Landscape => LANDSCAPE_BACKGROUND.view_rect(rectangle),
            Self::Portrait => PORTRAIT_BACKGROUND.view_rect(rectangle),
        }
    }
}

fn background_for_size(size: Size) -> Background {
    if size.width > size.height {
        Background::Landscape
    } else {
        Background::Portrait
    }
}

async fn draw_text_value(
    display: &mut CydDisplayWasm,
    background: Background,
    rectangle: Rectangle,
    text: String,
    color: Rgb888,
) -> Result<(), JsValue> {
    let mut frame = display.frame_mut(rectangle);
    DrawItem::Bitmap {
        view: background.view_rect(rectangle),
        top_left: rectangle.top_left,
    }
    .draw(&mut frame);
    let style = MonoTextStyle::new(&FONT_6X10, color.into());
    Text::with_baseline(text.as_str(), Point::zero(), style, Baseline::Top)
        .draw(&mut frame)
        .unwrap_infallible();
    frame.flush().await.map_err(|error| match error {})
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
        Control::Wifi => 1,
        Control::Calibration => 2,
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
    [Control::Orientation, Control::Wifi, Control::Calibration]
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
