const keyMap = new Map([
  [" ", "play_pause"],
  ["n", "next"],
  ["N", "next"],
  ["p", "prev"],
  ["P", "prev"],
  ["Escape", "cancel"],
]);

const panel = document.querySelector("#panel");
const status = document.querySelector("#status");
let imageUrl = null;
let conway = null;
let ConwayWeb = null;

async function render() {
  const pngBytes = conway.render_png();
  if (imageUrl) {
    URL.revokeObjectURL(imageUrl);
  }
  imageUrl = URL.createObjectURL(new Blob([pngBytes], { type: "image/png" }));
  panel.src = imageUrl;
}

async function tick() {
  if (!conway) {
    return;
  }
  status.textContent = conway.tick();
  await render();
}

document.addEventListener("keydown", async (event) => {
  const key = keyMap.get(event.key) ?? (/^[0-9]$/.test(event.key) ? event.key : null);
  if (!key) {
    return;
  }
  if (!conway) {
    return;
  }
  event.preventDefault();
  status.textContent = conway.press_key(key);
  await render();
});

for (const button of document.querySelectorAll("button[data-key]")) {
  button.addEventListener("click", async () => {
    if (!conway) {
      return;
    }
    status.textContent = conway.press_key(button.dataset.key);
    await render();
  });
}

try {
  status.textContent = "loading WASM module";
  const wasmModule = await import("./pkg/device_envoy_conway_wasm.js");
  const init = wasmModule.default;
  ConwayWeb = wasmModule.ConwayWeb;
  await init();
  conway = new ConwayWeb();
  status.textContent = "ready";
  await render();
  setInterval(tick, 180);
} catch (error) {
  console.error(error);
  status.textContent = `WASM load failed: ${error.message ?? error}`;
}

// Mouse coordinate display for calibrating remote button overlays
const remoteWrapper = document.querySelector(".remote-wrapper");
const remoteCoords = document.querySelector("#remote-coords");
remoteWrapper.addEventListener("mousemove", (e) => {
  const rect = remoteWrapper.getBoundingClientRect();
  const x = (((e.clientX - rect.left) / rect.width) * 100).toFixed(1);
  const y = (((e.clientY - rect.top) / rect.height) * 100).toFixed(1);
  remoteCoords.textContent = `${x}%, ${y}%`;
  remoteCoords.style.display = "block";
});
remoteWrapper.addEventListener("mouseleave", () => {
  remoteCoords.style.display = "none";
});
