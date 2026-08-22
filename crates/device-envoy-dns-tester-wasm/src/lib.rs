use core::convert::Infallible;

use device_envoy_core::wasm::cyd_web;
use device_envoy_core::{cyd::display::Orientation, wasm::WifiConnectOutcome};
use device_envoy_examples_core::dns_tester::{
    self as dns_tester, Error as CoreError, Exit as CoreExit,
};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::prelude::wasm_bindgen;

const BACKGROUND_COLOR: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND_COLOR: Rgb888 = Rgb888::new(230, 230, 230); // near-white

const WEB_APP: cyd_web::Config = cyd_web::Config::new(
    "device-envoy/dns-tester",
    Orientation::Landscape,
    BACKGROUND_COLOR,
    FOREGROUND_COLOR,
    &FONT_6X10,
);
const PAGE_INFO: cyd_web::PageInfo = cyd_web::PageInfo::new(
    "DNS Tester",
    "Measure a deterministic simulated DNS lookup on a CYD.",
    "The DNS tester exercises the shared device abstraction and reports a fixed browser simulation result.",
    "Touch the panel and press BOOT to interact with the tester.",
    "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<cyd_web::Handle, wasm_bindgen::JsValue> {
    cyd_web::start(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}

async fn inner_main(
    capabilities: cyd_web::Capabilities,
) -> Result<cyd_web::Command, CoreError<Infallible, Infallible>> {
    let mut cyd = capabilities.cyd;
    let mut button = capabilities.button;
    let wifi_simulator = capabilities.wifi_simulator;
    let mut dns_simulator = capabilities.dns_simulator;
    dns_tester::splash(&mut cyd).await?;

    if matches!(
        wifi_simulator
            .connect(&mut button, async |wifi_auto_event| {
                dns_tester::wifi_status(&mut cyd, wifi_auto_event).await
            })
            .await?,
        WifiConnectOutcome::ResetRequested
    ) {
        return Ok(cyd_web::Command::ResetWifi);
    }

    match dns_tester::run(&mut cyd, &mut button, &mut dns_simulator).await? {
        CoreExit::Calibrate => Ok(cyd_web::Command::CalibrationNotNeeded),
        CoreExit::ResetWifi => Ok(cyd_web::Command::ResetWifi),
        CoreExit::Reorientate(orientation) => Ok(cyd_web::Command::Reorientate(orientation)),
    }
}
