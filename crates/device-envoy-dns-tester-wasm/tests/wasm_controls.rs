use device_envoy_core::{
    cyd::display::Orientation,
    dns::{Dns as _, IpAddress},
    flash_block::FlashBlock as _,
    wasm::{
        CydDisplayWasm, CydSimulatorWasm, CydWebAppConfig, CydWebCommand, DnsFixedWasm,
        FlashBlockWasm, next_animation_frame, start_cyd_display_web_app, start_cyd_web_app,
    },
};
use device_envoy_dns_tester_wasm::start;
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{HtmlCanvasElement, window};

wasm_bindgen_test_configure!(run_in_browser);

fn canvas(document: &web_sys::Document, canvas_id: &str) -> Result<HtmlCanvasElement, JsValue> {
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_id(canvas_id);
    document
        .body()
        .ok_or_else(|| JsValue::from_str("body unavailable"))?
        .append_child(&canvas)?;
    Ok(canvas)
}

fn document() -> Result<web_sys::Document, JsValue> {
    window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .document()
        .ok_or_else(|| JsValue::from_str("document unavailable"))
}

#[wasm_bindgen_test]
fn shared_simulator_constructs_the_intrinsic_canvas() -> Result<(), JsValue> {
    let document = document()?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let simulator = CydSimulatorWasm::new(canvas.clone(), Orientation::Portrait)?;
    let (_cyd, _button, control) = simulator.into_parts();

    assert_eq!(canvas.width(), Orientation::Portrait.width());
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    let storage = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("local storage unavailable"))?;
    for index in 0..storage.length()? {
        let key = storage
            .key(index)?
            .ok_or_else(|| JsValue::from_str("storage key disappeared"))?;
        assert!(!key.contains("calibration"));
    }
    control.touch_down(120.0, 294.0);
    control.touch_up();
    control.reset_transient_state();
    Ok(())
}

#[wasm_bindgen_test]
fn touch_app_starts_in_saved_orientation_without_calibration_storage() -> Result<(), JsValue> {
    let document = document()?;
    let canvas = canvas(&document, "screen-orientation")?;
    let mut orientation_flash_block = FlashBlockWasm::new("device-envoy/dns-tester/orientation")
        .map_err(|error| JsValue::from_str(&format!("orientation flash: {error:?}")))?;
    orientation_flash_block
        .clear()
        .map_err(|error| JsValue::from_str(&format!("orientation clear: {error:?}")))?;
    orientation_flash_block
        .save(&Orientation::Portrait)
        .map_err(|error| JsValue::from_str(&format!("orientation save: {error:?}")))?;

    let handle = start("screen-orientation")?;
    assert_eq!(canvas.width(), Orientation::Portrait.width());
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    handle.touch_down(160.0, 216.0);
    handle.touch_up();
    handle.request_restart();
    Ok(())
}

#[wasm_bindgen_test]
async fn calibration_not_needed_queues_notice_and_restarts_stably() -> Result<(), JsValue> {
    let document = document()?;
    let canvas = canvas(&document, "screen-calibration-policy")?;
    let config = CydWebAppConfig::new(
        "device-envoy/dns-tester/calibration-policy",
        Orientation::Portrait,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let invocation_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let callback_invocation_count = invocation_count.clone();
    let handle = start_cyd_web_app("screen-calibration-policy", config, async move |_, _| {
        let invocation = callback_invocation_count.get();
        callback_invocation_count.set(invocation + 1);
        Ok::<CydWebCommand, core::convert::Infallible>(if invocation == 0 {
            CydWebCommand::CalibrationNotNeeded
        } else {
            CydWebCommand::Stop
        })
    })?;
    for _ in 0..5 {
        next_animation_frame().await;
    }
    let notice = handle
        .take_notice()
        .ok_or_else(|| JsValue::from_str("calibration policy notice was not queued"))?;
    assert_eq!(notice.id(), "calibration-not-needed");
    assert_eq!(
        notice.severity(),
        device_envoy_core::wasm::CydWebNoticeSeverity::Info
    );
    assert_eq!(canvas.height(), Orientation::Portrait.height());
    for _ in 0..5 {
        next_animation_frame().await;
    }
    assert_eq!(invocation_count.get(), 2);
    assert!(handle.take_notice().is_none());
    handle.touch_up();
    handle.boot_up();
    Ok(())
}

#[wasm_bindgen_test]
async fn display_app_uses_only_orientation_storage() -> Result<(), JsValue> {
    let document = document()?;
    let canvas = canvas(&document, "screen-display-only")?;
    let namespace = "device-envoy/display-only-storage-test";
    let config = CydWebAppConfig::new(
        namespace,
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let _handle = start_cyd_display_web_app(
        "screen-display-only",
        config,
        async |_: &mut CydDisplayWasm, _| {
            Ok::<CydWebCommand, core::convert::Infallible>(CydWebCommand::Stop)
        },
    )?;
    for _ in 0..3 {
        next_animation_frame().await;
    }
    let storage = window()
        .ok_or_else(|| JsValue::from_str("window unavailable"))?
        .local_storage()?
        .ok_or_else(|| JsValue::from_str("local storage unavailable"))?;
    assert!(
        storage
            .get_item(&format!("{namespace}/calibration"))?
            .is_none()
    );
    assert!(canvas.width() > 0);
    Ok(())
}

#[wasm_bindgen_test]
async fn fixed_dns_waits_for_simulated_latency() -> Result<(), JsValue> {
    let mut dns = DnsFixedWasm::new([IpAddress::Ipv4([127, 0, 0, 1].into())]);
    let started = embassy_time::Instant::now();
    let addresses = match dns.resolve("example.com").await {
        Ok(addresses) => addresses,
        Err(error) => match error {},
    };
    assert_eq!(addresses.len(), 1);
    assert!(started.elapsed() >= embassy_time::Duration::from_millis(12));
    Ok(())
}

#[wasm_bindgen_test]
async fn framework_fatal_notice_stops_and_preserves_diagnostic() -> Result<(), JsValue> {
    let document = document()?;
    let canvas = canvas(&document, "screen-fatal")?;
    let config = CydWebAppConfig::new(
        "device-envoy/dns-tester/fatal-test",
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let handle = start_cyd_web_app("screen-fatal", config, async |_, _| {
        Err::<CydWebCommand, _>("intentional fatal test error")
    })?;
    for _ in 0..5 {
        next_animation_frame().await;
    }
    let notice = handle
        .take_notice()
        .ok_or_else(|| JsValue::from_str("fatal notice was not queued"))?;
    assert_eq!(notice.id(), "runtime-error");
    assert_eq!(
        notice.detail().as_deref(),
        Some("application failed: \"intentional fatal test error\"")
    );
    assert_eq!(canvas.width(), Orientation::Landscape.width());
    Ok(())
}
