use device_envoy_core::{
    cyd::{display::Orientation, touch::calibration::CalibrationConfig},
    flash_block::FlashBlock as _,
    wasm::{next_animation_frame, FlashBlockWasm},
};
use device_envoy_dns_tester_wasm::DnsTesterWeb;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{window, HtmlCanvasElement};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn wrapper_forwards_rotation_and_boot_calibration_flow() -> Result<(), JsValue> {
    let document = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let calibration = CalibrationConfig::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0);
    let mut calibration_flash_block = FlashBlockWasm::new("device-envoy/dns-tester/calibration")
        .map_err(|error| JsValue::from_str(&format!("calibration flash: {error:?}")))?;
    calibration_flash_block
        .clear()
        .map_err(|error| JsValue::from_str(&format!("calibration clear: {error:?}")))?;
    calibration_flash_block
        .save(&calibration)
        .map_err(|error| JsValue::from_str(&format!("calibration save: {error:?}")))?;
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
    next_animation_frame().await;
    assert_eq!(tester.take_exit(), "orientation");
    assert!(tester.orientation_is_inverted());

    tester.boot_down();
    next_animation_frame().await;
    assert_eq!(tester.take_exit(), "recalibrate");
    tester.boot_up();
    tester.prepare_calibration_landscape();
    assert_eq!(canvas.width(), Orientation::Landscape.width());
    assert_eq!(canvas.height(), Orientation::Landscape.height());

    calibration_flash_block
        .save(&calibration)
        .map_err(|error| JsValue::from_str(&format!("calibration restore: {error:?}")))?;
    tester.reboot().await?;
    assert_eq!(canvas.width(), Orientation::Portrait.width());
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    assert!(!tester.orientation_is_inverted());
    Ok(())
}
