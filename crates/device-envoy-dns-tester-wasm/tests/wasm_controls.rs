use device_envoy_core::{
    cyd::display::Orientation,
    flash_block::FlashBlock as _,
    wasm::{CydSimulatorWasm, FlashBlockWasm, next_animation_frame},
};
use device_envoy_dns_tester_wasm::DnsTesterWeb;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlCanvasElement, window};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn shared_simulator_constructs_the_intrinsic_canvas() -> Result<(), JsValue> {
    let document = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let simulator = CydSimulatorWasm::new(canvas.clone(), Orientation::Portrait)?;
    let (_cyd, _button, control) = simulator.into_parts();

    assert_eq!(canvas.width(), Orientation::Portrait.width());
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    assert_eq!(control.orientation(), Orientation::Portrait);
    control.touch_down(120.0, 294.0);
    control.touch_up();
    control.reset_transient_state();
    Ok(())
}

#[wasm_bindgen_test(async)]
async fn wrapper_forwards_rotation_without_calibration_or_wifi_reset() -> Result<(), JsValue> {
    let document = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let mut orientation_flash_block = FlashBlockWasm::new("device-envoy/dns-tester/orientation")
        .map_err(|error| JsValue::from_str(&format!("orientation flash: {error:?}")))?;
    orientation_flash_block
        .clear()
        .map_err(|error| JsValue::from_str(&format!("orientation clear: {error:?}")))?;
    orientation_flash_block
        .save(&Orientation::Portrait)
        .map_err(|error| JsValue::from_str(&format!("orientation save: {error:?}")))?;

    let tester = DnsTesterWeb::new(canvas.clone())?;
    tester.start().await?;
    assert_eq!(canvas.width(), Orientation::Portrait.width());
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    assert!(!tester.orientation_is_inverted());

    tester.touch_down(193.0, 294.0);
    tester.touch_up();
    let mut exit = String::from("idle");
    for _ in 0..12 {
        next_animation_frame().await;
        exit = tester.take_exit();
        if exit != "idle" {
            break;
        }
    }
    assert_eq!(exit, "orientation");
    assert!(tester.orientation_is_inverted());
    assert_eq!(canvas.width(), Orientation::LandscapeInverted.width());
    assert_eq!(canvas.height(), Orientation::LandscapeInverted.height());
    tester.reboot().await?;

    Ok(())
}
