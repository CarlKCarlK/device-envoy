use core::convert::Infallible;

use device_envoy_core::{
    cyd::display::Orientation,
    wasm::{
        CydWebAppConfig, CydWebAppHandle, CydWebAppWasm, CydWebCommand, CydWebPageInfo,
        WifiConnectOutcome, start_cyd_web_app,
    },
};
use device_envoy_examples_core::dns_tester::{
    self as dns_tester, Error as CoreError, Exit as CoreExit,
};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::Rgb888};
use wasm_bindgen::prelude::wasm_bindgen;

const BACKGROUND: Rgb888 = Rgb888::new(10, 10, 12); // near-black
const FOREGROUND: Rgb888 = Rgb888::new(230, 230, 230); // near-white

const WEB_APP: CydWebAppConfig = CydWebAppConfig::new(
    "device-envoy/dns-tester",
    Orientation::Landscape,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
);
const PAGE_INFO: CydWebPageInfo = CydWebPageInfo::new(
    "DNS Tester",
    "Measure a deterministic simulated DNS lookup on a CYD.",
    "The DNS tester exercises the shared device abstraction and reports a fixed browser simulation result.",
    "Touch the panel and press BOOT to interact with the tester.",
    "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, wasm_bindgen::JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}

async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<CydWebCommand, CoreError<Infallible, Infallible>> {
    dns_tester::splash(&mut cyd_web_app_wasm.cyd).await?;

    if matches!(
        cyd_web_app_wasm
            .wifi_simulator
            .connect(&mut cyd_web_app_wasm.button, async |wifi_auto_event| {
                dns_tester::wifi_status(&mut cyd_web_app_wasm.cyd, wifi_auto_event).await
            })
            .await?,
        WifiConnectOutcome::ResetRequested
    ) {
        return Ok(CydWebCommand::ResetWifi);
    }

    match dns_tester::run(
        &mut cyd_web_app_wasm.cyd,
        &mut cyd_web_app_wasm.button,
        &mut cyd_web_app_wasm.dns_simulator,
    )
    .await?
    {
        CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded),
        CoreExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
        CoreExit::Reorientate(orientation) => Ok(CydWebCommand::Reorientate(orientation)),
    }
}
