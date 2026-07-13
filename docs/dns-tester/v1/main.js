import init, { DnsTesterWeb } from "./pkg/device_envoy_dns_tester_wasm.js?v=3";
import { setupDemoUx } from "./demo-ux.js";

const canvas = document.querySelector("#screen");
const stage = document.querySelector(".stage");
const boot = document.querySelector("#boot-button");
let tester;

function syncStage() {
  const isPortrait = canvas.height > canvas.width;
  stage.dataset.orientation = isPortrait ? "portrait" : "landscape";
  stage.dataset.inverted = tester.orientation_is_inverted() ? "true" : "false";
  window.dispatchEvent(new Event("resize"));
}

function point(event) {
  const bounds = canvas.getBoundingClientRect();
  let x = (event.clientX - bounds.left) * canvas.width / bounds.width;
  let y = (event.clientY - bounds.top) * canvas.height / bounds.height;
  if (stage.dataset.inverted === "true") {
    x = canvas.width - x;
    y = canvas.height - y;
  }
  return [x, y];
}

function enqueuePressSamples(x, y) {
  // Hardware naturally reports several samples during a press. A quick
  // browser tap may otherwise produce only Down/Up, which is intentionally
  // insufficient for the shared calibration driver's sample threshold.
  tester.touch_down(x, y);
  for (let sampleIndex = 0; sampleIndex < 8; sampleIndex += 1) {
    tester.touch_move(x, y);
  }
}

async function refresh() {
  await new Promise((resolve) => requestAnimationFrame(resolve));
  const result = tester.take_exit();
  if (result === "recalibrate") {
    tester.prepare_calibration_landscape();
    syncStage();
    await tester.reboot();
    syncStage();
  } else if (result === "orientation") {
    await tester.reboot();
    syncStage();
  }
}

try {
  await init();
  tester = new DnsTesterWeb(canvas);
  canvas.addEventListener("pointerdown", (event) => { const [x, y] = point(event); event.preventDefault(); canvas.setPointerCapture(event.pointerId); enqueuePressSamples(x, y); });
  canvas.addEventListener("pointermove", (event) => { if (event.buttons) { const [x, y] = point(event); tester.touch_move(x, y); } });
  canvas.addEventListener("pointerup", () => { tester.touch_up(); void refresh(); });
  boot.addEventListener("pointerdown", () => tester.boot_down());
  boot.addEventListener("pointerup", async () => { tester.boot_up(); await refresh(); });
  await tester.start();
  syncStage();
  setupDemoUx({
    title: "DNS Tester",
    orientation: "landscape",
    previewLine: "Exercise CYD touch, calibration, orientation, and reset behavior in your browser.",
    descriptionHtml: "<p>A browser companion to the Device Envoy hardware DNS tester. It uses a deterministic DNS result because browsers cannot issue arbitrary DNS requests directly.</p>",
    controlsHtml: "<p>Tap the screen to run a test. ROT cycles orientation, CAL clears calibration, WiFi is unavailable, and boot provides the physical-button recalibration path.</p>",
    coreCodeUrl: "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
  });
} catch (error) {
  console.error(error);
}
