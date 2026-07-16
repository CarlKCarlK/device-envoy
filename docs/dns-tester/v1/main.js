import init, { start } from "./pkg/device_envoy_dns_tester_wasm.js";
import { mountCydSimulator } from "./cyd-simulator.js";

try {
  await init();
  await mountCydSimulator({
    wasm: { start },
    app: {
      orientation: "landscape",
      touchDownSamples: 9,
      noticeMessages: {
        "runtime-error": "The DNS Tester simulator stopped because of a runtime error.",
      },
    },
  });
} catch (error) {
  console.error(error);
}
