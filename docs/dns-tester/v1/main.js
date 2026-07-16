import init, { DnsTesterWeb } from "./pkg/device_envoy_dns_tester_wasm.js?v=36c98fba5fa0";
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

const SIMULATOR_NOTICES = {
  "calibration-unavailable": {
    severity: "info",
    message: "Touch calibration is available on physical CYD hardware only.",
    durationMs: 3500,
  },
  "wifi-reset-unavailable": {
    severity: "info",
    message: "Wi-Fi reset is available on physical CYD hardware only.",
    durationMs: 3500,
  },
  "runtime-error": {
    severity: "fatal",
    message: "The DNS Tester simulator stopped because of a runtime error.",
    durationMs: 0,
  },
};

async function monitorRuntime(syncPresentation, showNotice) {
  while (true) {
    await nextFrame();
    const result = tester.take_exit();
    if (result === "orientation") {
      // take_exit() has already advanced the saved orientation and intrinsic
      // canvas dimensions. Apply that presentation before reboot draws the
      // splash in the new orientation.
      syncPresentation();
      await rebootAndSyncStage(syncPresentation);
    } else if (result === "calibration unavailable") {
      await rebootAndSyncStage(syncPresentation);
      showNotice(SIMULATOR_NOTICES["calibration-unavailable"]);
    } else if (result === "wifi reset unavailable") {
      await rebootAndSyncStage(syncPresentation);
      showNotice(SIMULATOR_NOTICES["wifi-reset-unavailable"]);
    } else if (result === "runtime error") {
      return;
    }
  }
}

try {
  await init({
    module_or_path: new URL("./pkg/device_envoy_dns_tester_wasm_bg.wasm?v=36c98fba5fa0", import.meta.url),
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
      previewLine: "Exercise CYD touch, DNS queries, and orientation behavior in your browser.",
      descriptionHtml: "<p>A browser companion to the Device Envoy hardware DNS tester. It uses a deterministic DNS result because browsers cannot issue arbitrary DNS requests directly.</p>",
      controlsHtml: "<p>Tap the screen to run a test. ROT changes orientation.</p>",
      coreCodeUrl: "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
    },
  });
  void monitorRuntime(syncPresentation, showNotice);
} catch (error) {
  console.error(error);
}
