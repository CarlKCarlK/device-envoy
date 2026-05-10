const keyMap = new Map([
  [" ", "play_pause"],
  ["n", "next"],
  ["N", "next"],
  ["p", "prev"],
  ["P", "prev"],
  ["Escape", "cancel"],
  ["m", "mode"],
  ["M", "mode"],
  ["r", "repeat"],
  ["R", "repeat"],
  ["u", "usd"],
  ["U", "usd"],
  ["]", "speed_up"],
  ["[", "speed_down"],
]);

const panel = document.querySelector("#panel");
const status = document.querySelector("#status");
let imageUrl = null;
let conway = null;
let tickTimer = null;

async function render() {
  const pngBytes = conway.render_png();
  if (imageUrl) {
    URL.revokeObjectURL(imageUrl);
  }
  imageUrl = URL.createObjectURL(new Blob([pngBytes], { type: "image/png" }));
  panel.src = imageUrl;
}

function restartTimer() {
  if (tickTimer !== null) {
    clearInterval(tickTimer);
  }
  tickTimer = setInterval(tick, conway.tick_interval_ms());
}

async function tick() {
  if (!conway) {
    return;
  }
  status.textContent = conway.tick();
  await render();
}

async function handleKey(key) {
  if (!conway) return;
  status.textContent = conway.press_key(key);
  if (key === "speed_up" || key === "speed_down") {
    restartTimer();
  }
  await render();
}

document.addEventListener("keydown", async (event) => {
  const key = keyMap.get(event.key) ?? (/^[0-9]$/.test(event.key) ? event.key : null);
  if (!key) return;
  event.preventDefault();
  await handleKey(key);
});

for (const button of document.querySelectorAll("button[data-key]")) {
  button.addEventListener("click", async () => {
    await handleKey(button.dataset.key);
  });
}

try {
  status.textContent = "loading WASM module";
  const wasmModule = await import("./pkg/device_envoy_conway_wasm.js?v=17");
  await wasmModule.default();
  conway = new wasmModule.ConwayWeb();
  status.textContent = "ready";
  await render();
  restartTimer();
} catch (error) {
  console.error(error);
  status.textContent = `WASM load failed: ${error.message ?? error}`;
}
