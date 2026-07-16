import init, { start } from "./pkg/device_envoy_dns_tester_wasm.js";
import { mountCydSimulator } from "./cyd-simulator.js";

try {
  await init();
  await mountCydSimulator({
    wasm: { start },
    app: {
      title: "DNS Tester",
      orientation: "landscape",
      touchDownSamples: 9,
      previewLine: "Exercise CYD touch, DNS queries, and orientation behavior in your browser.",
      descriptionHtml: "<p>A browser companion to the Device Envoy hardware DNS tester. It uses a deterministic DNS result because browsers cannot issue arbitrary DNS requests directly.</p>",
      controlsHtml: "<p>Tap the screen to run a test. ROT changes orientation.</p>",
      coreCodeUrl: "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
      noticeMessages: {
        "runtime-error": "The DNS Tester simulator stopped because of a runtime error.",
      },
    },
  });
} catch (error) {
  console.error(error);
}
