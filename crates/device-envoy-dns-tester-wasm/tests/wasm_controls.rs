use device_envoy_core::wasm::cyd_web;
use device_envoy_core::{
    clock_sync::ClockSync as _,
    cyd::display::Orientation,
    dns::Dns as _,
    flash_block::FlashBlock as _,
    wasm::{
        ClockSyncWasm, CydSimulatorWasm, DnsSimulatorWasm, FlashBlockWasm, next_animation_frame,
    },
};
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

    let config = cyd_web::Config::new(
        "device-envoy/dns-tester",
        Orientation::Portrait,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let page_info = cyd_web::PageInfo::new("Test", "Test", "Test", "Test", "https://example.com");
    let handle = cyd_web::start(
        "screen-orientation",
        config,
        page_info,
        async |_application: cyd_web::Capabilities| {
            Ok::<cyd_web::Command, core::convert::Infallible>(cyd_web::Command::Stop)
        },
    )?;
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
    let config = cyd_web::Config::new(
        "device-envoy/dns-tester/calibration-policy",
        Orientation::Portrait,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let invocation_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let callback_invocation_count = invocation_count.clone();
    let page_info = cyd_web::PageInfo::new("Test", "Test", "Test", "Test", "https://example.com");
    let handle = cyd_web::start(
        "screen-calibration-policy",
        config,
        page_info,
        async move |_application: cyd_web::Capabilities| {
            let invocation = callback_invocation_count.get();
            callback_invocation_count.set(invocation + 1);
            Ok::<cyd_web::Command, core::convert::Infallible>(if invocation == 0 {
                cyd_web::Command::CalibrationNotNeeded
            } else {
                cyd_web::Command::Stop
            })
        },
    )?;
    for _ in 0..5 {
        next_animation_frame().await;
    }
    let notice = handle
        .take_notice()
        .ok_or_else(|| JsValue::from_str("calibration policy notice was not queued"))?;
    assert_eq!(notice.id(), "calibration-not-needed");
    assert_eq!(notice.severity(), cyd_web::NoticeSeverity::Info);
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
    let config = cyd_web::Config::new(
        namespace,
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let page_info = cyd_web::PageInfo::new("Test", "Test", "Test", "Test", "https://example.com");
    let _handle = cyd_web::start(
        "screen-display-only",
        config,
        page_info,
        async |application: cyd_web::Capabilities| {
            let _display = application.cyd.display();
            Ok::<cyd_web::Command, core::convert::Infallible>(cyd_web::Command::Stop)
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
    let mut dns = DnsSimulatorWasm::standard();
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
    let config = cyd_web::Config::new(
        "device-envoy/dns-tester/fatal-test",
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let page_info = cyd_web::PageInfo::new("Test", "Test", "Test", "Test", "https://example.com");
    let handle = cyd_web::start(
        "screen-fatal",
        config,
        page_info,
        async |_application: cyd_web::Capabilities| {
            Err::<cyd_web::Command, _>("intentional fatal test error")
        },
    )?;
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

#[wasm_bindgen_test]
fn clock_control_is_instance_local_and_validates_time() -> Result<(), JsValue> {
    let first = ClockSyncWasm::new();
    let second = ClockSyncWasm::new();
    assert!(!first.control_is_visible());
    first.show();
    first.show();
    assert!(first.control_is_visible());
    assert!(!second.control_is_visible());
    assert!(first.now_local().time().hour() <= 23);
    Ok(())
}

#[wasm_bindgen_test]
async fn handle_clock_state_and_page_info_survive_restart() -> Result<(), JsValue> {
    let document = document()?;
    let _canvas = canvas(&document, "screen-clock-state")?;
    let config = cyd_web::Config::new(
        "device-envoy/clock-state-test",
        Orientation::Landscape,
        Rgb888::new(10, 10, 12),    // near-black
        Rgb888::new(230, 230, 230), // near-white
        &FONT_6X10,
    );
    let page_info = cyd_web::PageInfo::new(
        "Clock test",
        "Preview",
        "Description",
        "Controls",
        "https://example.com/core.rs",
    );
    let second_run_time = std::rc::Rc::new(std::cell::Cell::new(None));
    let second_run_time_ref = second_run_time.clone();
    let live_run_time = std::rc::Rc::new(std::cell::Cell::new(None));
    let live_run_time_ref = live_run_time.clone();
    let invocation_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let invocation_count_ref = invocation_count.clone();
    let handle = cyd_web::start(
        "screen-clock-state",
        config,
        page_info,
        async move |application: cyd_web::Capabilities| {
            let invocation = invocation_count_ref.get();
            invocation_count_ref.set(invocation + 1);
            if invocation == 0 {
                application.clock_sync.show();
                core::future::pending::<()>().await;
                unreachable!()
            }
            if invocation == 1 {
                second_run_time_ref.set(Some(application.clock_sync.now_local().time()));
                core::future::pending::<()>().await;
                unreachable!()
            }
            live_run_time_ref.set(Some(application.clock_sync.now_local().time()));
            Ok::<cyd_web::Command, core::convert::Infallible>(cyd_web::Command::Stop)
        },
    )?;
    for _ in 0..3 {
        next_animation_frame().await;
    }
    assert!(handle.clock_control_is_visible());
    handle.set_clock_time_of_day(43_200)?;
    assert!(handle.set_clock_time_of_day(86_400).is_err());
    assert_eq!(handle.page_title(), "Clock test");
    assert_eq!(handle.page_preview(), "Preview");
    assert_eq!(handle.page_description(), "Description");
    assert_eq!(handle.page_controls(), "Controls");
    assert_eq!(handle.page_core_code_url(), "https://example.com/core.rs");
    handle.request_restart();
    for _ in 0..6 {
        next_animation_frame().await;
    }
    assert_eq!(invocation_count.get(), 2);
    let time = second_run_time
        .get()
        .ok_or_else(|| JsValue::from_str("second run did not start"))?;
    assert_eq!(time.hour(), 12);
    assert_eq!(time.minute(), 0);
    assert!(handle.clock_control_is_visible());
    handle.use_live_clock();
    handle.request_restart();
    for _ in 0..6 {
        next_animation_frame().await;
    }
    let live_time = live_run_time
        .get()
        .ok_or_else(|| JsValue::from_str("live-clock run did not start"))?;
    assert!(live_time != time);
    Ok(())
}
