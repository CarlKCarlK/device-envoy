import init, { DnsTesterWeb } from "./pkg/device_envoy_dns_tester_wasm.js?v=6";
import { setupDemoUx } from "./demo-ux.js";

const canvas = document.querySelector("#screen");
const stage = document.querySelector(".stage");
const boot = document.querySelector("#boot-button");
let tester;
let canvasSizeObserver;

function nextFrame() {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

function syncStage() {
  const isPortrait = canvas.height > canvas.width;
  const isInverted = tester.orientation_is_inverted();
  stage.dataset.orientation = isPortrait ? "portrait" : "landscape";
  stage.dataset.inverted = isInverted ? "true" : "false";
  canvas.dataset.inverted = isInverted ? "true" : "false";
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

function showWifiUnavailableNotice() {
  const notice = document.createElement("div");
  notice.className = "simulation-notice";
  notice.textContent = "Wi-Fi setup is not available in the browser simulation.";
  document.body.append(notice);
  window.setTimeout(() => notice.remove(), 2200);
}

async function rebootAndSyncStage() {
  const reboot = tester.reboot();
  await reboot;
  syncStage();
}

async function monitorRuntime() {
  while (true) {
    await nextFrame();
    const result = tester.take_exit();
    if (result === "recalibrate") {
      // A browser pointer can remain down while the async reset begins. A
      // physical reset starts with a fresh button sample, so release it here.
      tester.boot_up();
      tester.prepare_calibration_landscape();
      syncStage();
      await rebootAndSyncStage();
    } else if (result === "orientation") {
      // take_exit() has already advanced the saved orientation and intrinsic
      // canvas dimensions. Apply that presentation before reboot draws the
      // splash in the new orientation.
      syncStage();
      await rebootAndSyncStage();
    } else if (result === "wifi") {
      showWifiUnavailableNotice();
      await rebootAndSyncStage();
    } else if (result === "runtime error") {
      return;
    }
  }
}

try {
  await init({
    module_or_path: new URL("./pkg/device_envoy_dns_tester_wasm_bg.wasm?v=6", import.meta.url),
  });
  tester = new DnsTesterWeb(canvas);
  canvasSizeObserver = new MutationObserver(syncStage);
  canvasSizeObserver.observe(canvas, {
    attributes: true,
    attributeFilter: ["width", "height"],
  });
  canvas.addEventListener("pointerdown", (event) => { const [x, y] = point(event); event.preventDefault(); canvas.setPointerCapture(event.pointerId); enqueuePressSamples(x, y); });
  canvas.addEventListener("pointermove", (event) => { if (event.buttons) { const [x, y] = point(event); tester.touch_move(x, y); } });
  canvas.addEventListener("pointerup", () => { tester.touch_up(); });
  canvas.addEventListener("pointercancel", () => { tester.touch_up(); });
  boot.addEventListener("pointerdown", () => tester.boot_down());
  boot.addEventListener("pointerup", () => { tester.boot_up(); });
  boot.addEventListener("pointercancel", () => { tester.boot_up(); });
  await tester.start();
  syncStage();
  void monitorRuntime();
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
