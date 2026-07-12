// Master copy: crates/linkage-blaze-clock-wasm/www/demo-ux.js
// TODO unify the four copies of demo-ux.js/css into one shared source.

const SHARED_SMALL_PRINT =
  "The simulation is the same no_std, no-allocation Rust that runs on a ~$15 ESP32-2432S028R - the \"Cheap Yellow Display\" (CYD). Here the core is compiled to WebAssembly; on a real desk it drives a 320x240 touch panel.";
const SCENE_MARGIN_X = 48;
const SCENE_MARGIN_TOP = 88;
const SCENE_MARGIN_BOTTOM = 104;
const SECONDS_PER_DAY = 86400;

export function setupDemoUx(config) {
  const simulator = requireElement(".simulator", HTMLDivElement);
  const stage = requireElement(".stage", HTMLDivElement);
  const canvas = requireElement("#screen", HTMLCanvasElement);
  const body = document.body;

  body.classList.add("demo-ux-page", `demo-ux-page--${config.orientation}`);
  if (config.timeSetter) {
    body.classList.add("demo-ux-page--has-time-setter");
  }

  const galleryTag = buildGalleryTag();
  const sceneCard = buildSceneCard(config);
  const deviceMode = buildDeviceMode({
    body,
    canvas,
    config,
    simulator,
    stage,
  });

  body.append(galleryTag.link, sceneCard.restingButton, sceneCard.scrim, deviceMode.button, deviceMode.overlay);

  if (config.timeSetter) {
    buildTimeSetter({
      body,
      setTimeOfDay: config.timeSetter.setTimeOfDay,
      openDeviceMode: () => deviceMode.isActive(),
    });
  }

  const updateSceneScale = () => {
    const previousTransform = simulator.style.transform;
    simulator.style.transform = "none";
    const naturalWidth = simulator.offsetWidth;
    const naturalHeight = simulator.offsetHeight;
    simulator.style.transform = previousTransform;

    if (naturalWidth === 0 || naturalHeight === 0) {
      return;
    }

    const availableWidth = Math.max(240, window.innerWidth - SCENE_MARGIN_X);
    const availableHeight = Math.max(
      240,
      window.innerHeight - SCENE_MARGIN_TOP - SCENE_MARGIN_BOTTOM,
    );
    const scale = Math.min(1, availableWidth / naturalWidth, availableHeight / naturalHeight);
    simulator.style.transform = `scale(${scale})`;
  };

  window.addEventListener("resize", updateSceneScale);
  window.requestAnimationFrame(updateSceneScale);
  window.setTimeout(updateSceneScale, 120);

  window.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") {
      return;
    }

    if (sceneCard.isOpen()) {
      sceneCard.close();
      return;
    }

    if (deviceMode.isActive()) {
      void deviceMode.deactivate();
    }
  });
}

function buildGalleryTag() {
  const link = document.createElement("a");
  link.className = "demo-ux-gallery-tag";
  link.href = "../../";
  link.textContent = "\u2190 Gallery";
  link.setAttribute("aria-label", "Back to gallery");
  return { link };
}

function buildSceneCard(config) {
  const restingButton = document.createElement("button");
  restingButton.type = "button";
  restingButton.className = "demo-ux-card-tag";
  restingButton.innerHTML = `
    <span class="demo-ux-card-tag__kicker">CYD demo</span>
    <strong>${escapeHtml(config.title)}</strong>
    <span class="demo-ux-card-tag__preview">${escapeHtml(config.previewLine)}</span>
    <span class="demo-ux-card-tag__hint">tap for details ›</span>
  `;

  const scrim = document.createElement("div");
  scrim.className = "demo-ux-card-scrim";
  scrim.hidden = true;

  const dialog = document.createElement("section");
  dialog.className = "demo-ux-card-dialog";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-labelledby", "demo-ux-card-title");
  dialog.tabIndex = -1;
  dialog.innerHTML = `
    <button class="demo-ux-card-close" type="button" aria-label="Close details">\u00d7</button>
    <p class="demo-ux-card-kicker">CYD demo</p>
    <h2 id="demo-ux-card-title">${escapeHtml(config.title)}</h2>
    <section class="demo-ux-card-section">
      <h3>What is this</h3>
      ${config.descriptionHtml}
    </section>
    <section class="demo-ux-card-section">
      <h3>Controls</h3>
      ${config.controlsHtml}
    </section>
    <section class="demo-ux-card-section">
      <h3>Links</h3>
      <div class="demo-ux-card-links">
        <a href="${config.coreCodeUrl}" target="_blank" rel="noopener">Core code</a>
        <a href="../../">Gallery</a>
        <a href="https://github.com/CarlKCarlK/linkage-blaze" target="_blank" rel="noopener">GitHub repo</a>
        <a href="https://medium.com/@carlmkadie" target="_blank" rel="noopener">Medium</a>
      </div>
    </section>
    <p class="demo-ux-card-smallprint">${escapeHtml(SHARED_SMALL_PRINT)}</p>
  `;
  scrim.append(dialog);

  let previousFocus = null;

  const open = () => {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    scrim.hidden = false;
    document.body.classList.add("demo-ux-card-open");
    dialog.focus();
  };

  const close = () => {
    if (scrim.hidden) {
      return;
    }
    scrim.hidden = true;
    document.body.classList.remove("demo-ux-card-open");
    previousFocus?.focus();
  };

  restingButton.addEventListener("click", open);
  scrim.addEventListener("click", (event) => {
    if (event.target === scrim) {
      close();
    }
  });
  dialog.querySelector(".demo-ux-card-close")?.addEventListener("click", close);

  return {
    restingButton,
    scrim,
    close,
    isOpen: () => !scrim.hidden,
  };
}

function buildDeviceMode({ body, canvas, config, simulator, stage }) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "demo-ux-device-button";
  button.textContent = "full-screen mode";

  const overlay = document.createElement("div");
  overlay.className = "demo-ux-device-overlay";
  overlay.hidden = true;

  const closeButton = document.createElement("button");
  closeButton.type = "button";
  closeButton.className = "demo-ux-device-close";
  closeButton.setAttribute("aria-label", "Exit full-screen mode");
  closeButton.textContent = "\u00d7";

  const screenHost = document.createElement("div");
  screenHost.className = "demo-ux-device-screen";
  overlay.append(closeButton, screenHost);

  let active = false;
  let exiting = false;
  let usedFullscreen = false;
  let canvasPlaceholder = null;

  const resizeDeviceCanvas = () => {
    if (!active) {
      return;
    }

    const pixelWidth = canvas.width || (config.orientation === "landscape" ? 320 : 240);
    const pixelHeight = canvas.height || (config.orientation === "landscape" ? 240 : 320);
    const aspectRatio = pixelWidth / pixelHeight;
    const availableWidth = overlay.clientWidth;
    const availableHeight = overlay.clientHeight;

    if (availableWidth === 0 || availableHeight === 0) {
      return;
    }

    let width = availableWidth;
    let height = width / aspectRatio;
    if (height > availableHeight) {
      height = availableHeight;
      width = height * aspectRatio;
    }

    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    canvas.style.left = "auto";
    canvas.style.top = "auto";
    canvas.style.position = "relative";
  };

  const restoreCanvas = () => {
    if (canvasPlaceholder?.parentNode) {
      canvasPlaceholder.replaceWith(canvas);
      canvasPlaceholder = null;
    }
    canvas.style.width = "";
    canvas.style.height = "";
    canvas.style.left = "";
    canvas.style.top = "";
    canvas.style.position = "";
  };

  const finishDeactivate = () => {
    restoreCanvas();
    overlay.hidden = true;
    body.classList.remove("demo-ux-device-active");
    simulator.removeAttribute("aria-hidden");
    stage.removeAttribute("aria-hidden");
    active = false;
    usedFullscreen = false;
    exiting = false;
  };

  const activate = async () => {
    if (active) {
      return;
    }

    active = true;
    overlay.hidden = false;
    body.classList.add("demo-ux-device-active");
    simulator.setAttribute("aria-hidden", "true");
    stage.setAttribute("aria-hidden", "true");

    canvasPlaceholder = document.createComment("demo-ux-canvas-placeholder");
    canvas.parentNode?.insertBefore(canvasPlaceholder, canvas);
    screenHost.append(canvas);
    resizeDeviceCanvas();

    if (typeof overlay.requestFullscreen === "function") {
      try {
        await overlay.requestFullscreen();
        usedFullscreen = document.fullscreenElement === overlay;
      } catch (_error) {
        usedFullscreen = false;
      }
    }
  };

  const deactivate = async () => {
    if (!active || exiting) {
      return;
    }

    exiting = true;
    if (usedFullscreen && document.fullscreenElement === overlay) {
      await document.exitFullscreen();
      if (active) {
        finishDeactivate();
      }
      return;
    }

    finishDeactivate();
  };

  button.addEventListener("click", () => {
    void activate();
  });
  closeButton.addEventListener("click", () => {
    void deactivate();
  });
  window.addEventListener("resize", resizeDeviceCanvas);
  document.addEventListener("fullscreenchange", () => {
    if (!active || !usedFullscreen || document.fullscreenElement === overlay || exiting) {
      return;
    }
    finishDeactivate();
  });

  return {
    button,
    overlay,
    deactivate,
    isActive: () => active,
  };
}

function buildTimeSetter({ body, setTimeOfDay, openDeviceMode }) {
  const chip = document.createElement("button");
  chip.type = "button";
  chip.className = "demo-ux-time-chip";

  const deviceChip = document.createElement("button");
  deviceChip.type = "button";
  deviceChip.className = "demo-ux-time-chip demo-ux-time-chip--device";

  const dock = document.createElement("section");
  dock.className = "demo-ux-time-dock";
  dock.hidden = true;

  const readout = document.createElement("div");
  readout.className = "demo-ux-time-readout";

  const range = document.createElement("input");
  range.className = "demo-ux-time-range";
  range.type = "range";
  range.min = "0";
  range.max = String(SECONDS_PER_DAY);
  range.step = "60";
  range.value = String(12 * 3600);

  const liveButton = document.createElement("button");
  liveButton.type = "button";
  liveButton.className = "demo-ux-time-action";
  liveButton.textContent = "Live";

  const collapseButton = document.createElement("button");
  collapseButton.type = "button";
  collapseButton.className = "demo-ux-time-action";
  collapseButton.textContent = "\u00d7";
  collapseButton.setAttribute("aria-label", "Collapse time control");

  dock.append(readout, range, liveButton, collapseButton);
  body.append(chip, deviceChip, dock);

  let live = true;

  const updateChipLabels = () => {
    const label = live ? "LIVE" : formatTime(Number(range.value));
    chip.innerHTML = `<span class="demo-ux-time-chip__icon">\u23f0</span><span>${label}</span>`;
    deviceChip.innerHTML = `<span class="demo-ux-time-chip__icon">\u23f0</span><span>${label}</span>`;
    readout.textContent = label;
  };

  const openDock = () => {
    dock.hidden = false;
    body.classList.add("demo-ux-time-open");
  };

  const closeDock = () => {
    dock.hidden = true;
    body.classList.remove("demo-ux-time-open");
  };

  const applyOverride = () => {
    live = false;
    setTimeOfDay(Number(range.value));
    updateChipLabels();
  };

  chip.addEventListener("click", openDock);
  deviceChip.addEventListener("click", openDock);
  range.addEventListener("input", applyOverride);
  liveButton.addEventListener("click", () => {
    live = true;
    setTimeOfDay(-1);
    updateChipLabels();
  });
  collapseButton.addEventListener("click", closeDock);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !dock.hidden) {
      closeDock();
    }
  });

  updateChipLabels();
  if (openDeviceMode()) {
    closeDock();
  }
}

function formatTime(secondsOfDay) {
  const hour = Math.floor(secondsOfDay / 3600);
  const minute = Math.floor((secondsOfDay % 3600) / 60);
  const suffix = hour < 12 ? "AM" : "PM";
  const hour12 = hour % 12 === 0 ? 12 : hour % 12;
  return `${hour12}:${String(minute).padStart(2, "0")} ${suffix}`;
}

function requireElement(selector, expectedType) {
  const element = document.querySelector(selector);
  if (!(element instanceof expectedType)) {
    throw new Error(`missing ${selector}`);
  }
  return element;
}

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;");
}
