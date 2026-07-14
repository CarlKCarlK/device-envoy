import init, { DnsTesterWeb } from "./pkg/device_envoy_dns_tester_wasm.js?v=52fc9c6b2e8c";
import { mountCydSimulator } from "./cyd-simulator.js";

const canvas = document.querySelector("#screen");
let tester;

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

async function rebootAndSyncStage(syncPresentation) {
  const reboot = tester.reboot();
  await reboot;
  syncPresentation();
}

async function monitorRuntime(syncPresentation, showNotice) {
  while (true) {
    await nextFrame();
    const result = tester.take_exit();
    if (result === "recalibrate") {
      // A browser pointer can remain down while the async reset begins. A
      // physical reset starts with a fresh button sample, so release it here.
      tester.boot_up();
      tester.prepare_calibration_landscape();
      syncPresentation();
      await rebootAndSyncStage(syncPresentation);
    } else if (result === "orientation") {
      // take_exit() has already advanced the saved orientation and intrinsic
      // canvas dimensions. Apply that presentation before reboot draws the
      // splash in the new orientation.
      syncPresentation();
      await rebootAndSyncStage(syncPresentation);
    } else if (result === "wifi") {
      showNotice({
        severity: "warning",
        message: "Wi-Fi setup is simulated; reconnecting from the captive-portal state.",
      });
      tester.boot_up();
      await rebootAndSyncStage(syncPresentation);
    } else if (result === "runtime error") {
      return;
    }
  }
}

try {
  await init({
    module_or_path: new URL("./pkg/device_envoy_dns_tester_wasm_bg.wasm?v=52fc9c6b2e8c", import.meta.url),
  });
  tester = new DnsTesterWeb(canvas);
  const { syncPresentation, showNotice } = await mountCydSimulator({
    wasm: {
      handle: tester,
      start: () => tester.start(),
    },
    app: {
      title: "DNS Tester",
      orientation: "landscape",
      touchDownSamples: 9,
      previewLine: "Exercise CYD touch, calibration, orientation, and reset behavior in your browser.",
      descriptionHtml: "<p>A browser companion to the Device Envoy hardware DNS tester. It uses a deterministic DNS result because browsers cannot issue arbitrary DNS requests directly.</p>",
      controlsHtml: "<p>Tap the screen to run a test. ROT cycles orientation, CAL clears calibration, WiFi restarts the simulated captive-portal connection, and boot provides the physical-button recalibration path.</p>",
      coreCodeUrl: "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
    },
  });
  void monitorRuntime(syncPresentation, showNotice);
} catch (error) {
  console.error(error);
}
