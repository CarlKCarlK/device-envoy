use std::{cell::Cell, rc::Rc};

use device_envoy_core::{
    cyd::{
        display::Orientation,
        touch::calibration::{CalibrationConfig, ensure_calibration},
    },
    flash_block::FlashBlock as _,
    wasm::{ButtonWasmSource, CydTouchWasmSource, CydWasm, FlashBlockWasm, next_animation_frame},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlCanvasElement, window};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn calibration_draws_and_saves_through_cyd_wasm() -> Result<(), JsValue> {
    let document = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(320);
    canvas.set_height(240);
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("2D context unavailable"))?
        .dyn_into()?;

    let touch_source = CydTouchWasmSource::new();
    let button_source = ButtonWasmSource::new();
    let device = CydWasm::new(
        context,
        Orientation::Landscape,
        embedded_graphics::pixelcolor::Rgb888::new(0, 0, 0),
        embedded_graphics::pixelcolor::Rgb888::new(255, 255, 255),
        &embedded_graphics::mono_font::ascii::FONT_6X10,
        touch_source.clone(),
    );
    let (mut display, uncalibrated_touch) = device.parts_uncalibrated();
    let mut flash_block = FlashBlockWasm::new("device-envoy/test/wasm-calibration")
        .map_err(|error| JsValue::from_str(&format!("flash: {error:?}")))?;
    flash_block
        .clear()
        .map_err(|error| JsValue::from_str(&format!("clear: {error:?}")))?;
    let mut button = button_source.button();
    let outcome = Rc::new(Cell::new(None::<bool>));
    let outcome_ref = outcome.clone();

    wasm_bindgen_futures::spawn_local(async move {
        let calibration_succeeded = match ensure_calibration(
            &mut display,
            uncalibrated_touch,
            &mut flash_block,
            &mut button,
            Some("Touch calibrated"),
        )
        .await
        {
            Ok((_, outcome)) => outcome.was_saved(),
            Err(_) => false,
        };
        outcome_ref.set(Some(calibration_succeeded));
    });

    // Let the calibration task draw its first target before the first press.
    next_animation_frame().await;
    for (x, y) in [(40.0, 40.0), (280.0, 40.0), (280.0, 200.0), (40.0, 200.0)] {
        press_with_samples(&touch_source, x, y);
        // The shared driver displays an acknowledgement for eight frames
        // before accepting the next corner.
        for _frame_index in 0..10 {
            next_animation_frame().await;
        }
    }
    press_with_samples(&touch_source, 160.0, 120.0);

    while outcome.get().is_none() {
        next_animation_frame().await;
    }
    let calibration_succeeded = outcome
        .get()
        .ok_or_else(|| JsValue::from_str("calibration did not finish"))?;
    assert!(calibration_succeeded);

    let mut saved_flash_block = FlashBlockWasm::new("device-envoy/test/wasm-calibration")
        .map_err(|error| JsValue::from_str(&format!("flash reload: {error:?}")))?;
    let saved_calibration = saved_flash_block
        .load::<CalibrationConfig>()
        .map_err(|error| JsValue::from_str(&format!("load: {error:?}")))?;
    assert!(saved_calibration.is_some());
    Ok(())
}

fn press_with_samples(touch_source: &CydTouchWasmSource, x: f32, y: f32) {
    touch_source.touch_down(x, y);
    for _sample_index in 0..8 {
        touch_source.touch_move(x, y);
    }
    touch_source.touch_up();
}
