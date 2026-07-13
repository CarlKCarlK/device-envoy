use device_envoy_core::{
    cyd::{
        display::Orientation,
        touch::calibration::{
            CalibrationConfig, CalibrationCorner, calibration_corner_center,
            distort_demo_screen_to_raw,
        },
    },
    flash_block::FlashBlock as _,
    wasm::{FlashBlockWasm, next_animation_frame},
};
use device_envoy_dns_tester_wasm::DnsTesterWeb;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlCanvasElement, window};

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
    let calibration = CalibrationConfig::from_four_points([
        demo_raw_calibration_point(0),
        demo_raw_calibration_point(1),
        demo_raw_calibration_point(2),
        demo_raw_calibration_point(3),
    ]);
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

    tester.boot_down();
    let mut exit = String::from("idle");
    for _ in 0..12 {
        next_animation_frame().await;
        exit = tester.take_exit();
        if exit != "idle" {
            break;
        }
    }
    assert_eq!(exit, "recalibrate");
    tester.prepare_calibration_landscape();
    assert_eq!(canvas.width(), Orientation::Landscape.width());
    assert_eq!(canvas.height(), Orientation::Landscape.height());

    calibration_flash_block
        .save(&calibration)
        .map_err(|error| JsValue::from_str(&format!("calibration restore: {error:?}")))?;
    tester.reboot().await?;
    assert_eq!(canvas.width(), Orientation::LandscapeInverted.width());
    assert_eq!(canvas.height(), Orientation::LandscapeInverted.height());
    assert!(tester.orientation_is_inverted());
    for _ in 0..3 {
        next_animation_frame().await;
    }
    assert_eq!(tester.take_exit(), "idle");
    Ok(())
}

fn demo_raw_calibration_point(corner_index: usize) -> device_envoy_core::cyd::touch::RawPoint {
    let calibration_corners = [
        CalibrationCorner::UpperLeft,
        CalibrationCorner::UpperRight,
        CalibrationCorner::LowerRight,
        CalibrationCorner::LowerLeft,
    ];
    let point = calibration_corner_center(calibration_corners[corner_index]);
    distort_demo_screen_to_raw(point.x as f32, point.y as f32)
}
