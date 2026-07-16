use core::convert::Infallible;

use device_envoy_core::{
    cyd::display::Orientation,
    dns::IpAddress,
    wasm::{
        ButtonWasm, CydWasm, CydWebAppConfig, CydWebAppHandle, CydWebCommand, DnsFixedWasm,
        WifiConnectOutcome, WifiSimulatorWasm, start_cyd_web_app,
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

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, wasm_bindgen::JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, inner_main)
}

async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, CoreError<Infallible, Infallible>> {
    dns_tester::splash(cyd).await?;

    let wifi_simulator = WifiSimulatorWasm::new(WEB_APP.storage_namespace);
    if matches!(
        wifi_simulator
            .connect(button, async |wifi_auto_event| {
                dns_tester::wifi_status(cyd, wifi_auto_event).await
            })
            .await?,
        WifiConnectOutcome::ResetRequested
    ) {
        return Ok(CydWebCommand::ResetWifi);
    }

    let mut dns = DnsFixedWasm::new([IpAddress::Ipv4([127, 0, 0, 1].into())]);
    match dns_tester::run(cyd, button, &mut dns).await? {
        CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded),
        CoreExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
        CoreExit::Reorientate(orientation) => Ok(CydWebCommand::Reorientate(orientation)),
    }
}
