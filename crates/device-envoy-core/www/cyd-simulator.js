// Canonical CYD browser shell. Application pages provide metadata and a small
// WASM handle; generic device and browser interaction stays here.

const SHARED_SMALL_PRINT =
  "The simulation is the same no_std, no-allocation Rust that runs on a ~$15 ESP32-2432S028R - the \"Cheap Yellow Display\" (CYD). Here the core is compiled to WebAssembly; on a real desk it drives a 320x240 touch panel.";
const SCENE_MARGIN_X = 48;
const SCENE_MARGIN_TOP = 88;
const SCENE_MARGIN_BOTTOM = 104;
const SECONDS_PER_DAY = 86399;

/** Mount the shared shell, start an application, and bind its input protocol. */
export async function mountCydSimulator({ wasm, app }) {
  const canvas = requireElement("#screen", HTMLCanvasElement);
  const { boot, stage } = ensurePhysicalShell(canvas);

  if (wasm.init) {
    await wasm.init();
  }
  let handle = wasm.handle ?? wasm;
  if (wasm.start) {
    handle = await wasm.start("screen");
  }
  const syncPresentation = () => {
    const isPortrait = canvas.height > canvas.width;
    const isInverted = typeof handle.orientation_is_inverted === "function"
      ? handle.orientation_is_inverted()
      : false;
    stage.dataset.orientation = isPortrait ? "portrait" : "landscape";
    stage.dataset.inverted = isInverted ? "true" : "false";
    canvas.dataset.inverted = isInverted ? "true" : "false";
    window.dispatchEvent(new Event("resize"));
  };

  const canvasSizeObserver = new MutationObserver(syncPresentation);
  canvasSizeObserver.observe(canvas, {
    attributes: true,
    attributeFilter: ["width", "height"],
  });
  const unbindInput = bindInputProtocol(canvas, boot, handle, app.touchDownSamples ?? 9);
  syncPresentation();
  const pageInfo = {
    title: handle.page_title(),
    previewLine: handle.page_preview(),
    description: handle.page_description(),
    controls: handle.page_controls(),
    coreCodeUrl: handle.page_core_code_url(),
  };
  const demoUx = setupDemoUx({ ...app, ...pageInfo, handle, orientation: app.orientation ?? "landscape" });
  monitorClockControl(handle, demoUx.showTimeControl);
  void monitorTypedNotices(handle, demoUx.showNotice, app.noticeMessages);
  return { handle, syncPresentation, showNotice: demoUx.showNotice };
}

async function monitorClockControl(handle, showTimeControl) {
  while (true) {
    await new Promise((resolve) => window.requestAnimationFrame(resolve));
    if (typeof handle.clock_control_is_visible !== "function") return;
    if (handle.clock_control_is_visible()) {
      showTimeControl();
      return;
    }
  }
}

async function monitorTypedNotices(handle, showNotice, noticeMessages = {}) {
  while (true) {
    await new Promise((resolve) => window.requestAnimationFrame(resolve));
    if (typeof handle.take_notice !== "function") {
      return;
    }
    let notice = handle.take_notice();
    while (notice) {
      const id = notice.id();
      const severityName = ["info", "warning", "fatal"][notice.severity()] ?? "fatal";
      const detail = typeof notice.detail === "function" ? notice.detail() : undefined;
      if (detail) {
        console.error(detail);
      }
      const message = noticeMessages[id] ?? defaultNoticeMessage(id);
      showNotice({
        severity: severityName,
        message,
        durationMs: id === "wifi-simulated" || severityName === "fatal" ? 0 : 3500,
      });
      notice = handle.take_notice();
    }
  }
}

function defaultNoticeMessage(id) {
  switch (id) {
    case "wifi-simulated":
      return "Wi-Fi connection is simulated in the browser.";
    case "wifi-setup":
      return "Wi-Fi setup is ready.";
    case "wifi-connecting":
      return "Connecting to Wi-Fi.";
    case "wifi-unavailable":
      return "Wi-Fi is unavailable.";
    case "runtime-error":
      return "The CYD simulator stopped because of a runtime error.";
    case "calibration-not-needed":
      return "Calibration is not needed in the browser.";
    default:
      return `CYD notice: ${id}`;
  }
}

function ensurePhysicalShell(canvas) {
  let simulator = canvas.closest(".simulator");
  if (!(simulator instanceof HTMLDivElement)) {
    simulator = document.createElement("div");
    simulator.className = "simulator";
    canvas.replaceWith(simulator);
    simulator.append(canvas);
  }

  let stage = canvas.closest(".stage");
  if (!(stage instanceof HTMLDivElement)) {
    stage = document.createElement("div");
    stage.className = "stage";
    canvas.replaceWith(stage);
    stage.append(canvas);
  }

  let cord = stage.querySelector(".cord");
  if (!(cord instanceof HTMLDivElement)) {
    cord = document.createElement("div");
    cord.className = "cord";
    stage.prepend(cord);
  }

  let caseImage = stage.querySelector(".case");
  if (!(caseImage instanceof HTMLImageElement)) {
    caseImage = document.createElement("img");
    caseImage.className = "case";
    caseImage.src = "./case.png";
    caseImage.alt = "CYD device case";
    stage.insertBefore(caseImage, canvas);
  }

  let boot = stage.querySelector("#boot-button");
  if (!(boot instanceof HTMLButtonElement)) {
    boot = document.createElement("button");
    boot.id = "boot-button";
    boot.className = "boot-button";
    boot.type = "button";
    boot.textContent = "boot";
  }
  stage.append(boot);
  return { simulator, stage, boot };
}

function bindInputProtocol(canvas, boot, handle, touchDownSamples) {
  const listeners = [];
  let touchActive = false;
  let bootActive = false;
  const listen = (target, type, listener, options) => {
    target.addEventListener(type, listener, options);
    listeners.push(() => target.removeEventListener(type, listener, options));
  };
  const invoke = (name, ...arguments_) => {
    if (typeof handle[name] === "function") {
      return handle[name](...arguments_);
    }
    return undefined;
  };
  const point = (event) => {
    const bounds = canvas.getBoundingClientRect();
    let point = [
      (event.clientX - bounds.left) * canvas.width / bounds.width,
      (event.clientY - bounds.top) * canvas.height / bounds.height,
    ];
    if (canvas.dataset.inverted === "true") {
      point = [canvas.width - point[0], canvas.height - point[1]];
    }
    return point;
  };

  const releaseTouch = () => {
    if (!touchActive) {
      return;
    }
    touchActive = false;
    invoke("touch_up");
  };
  const releaseBoot = () => {
    if (!bootActive) {
      return;
    }
    bootActive = false;
    invoke("boot_up");
  };

  listen(canvas, "pointerdown", (event) => {
    const [x, y] = point(event);
    event.preventDefault();
    canvas.setPointerCapture(event.pointerId);
    touchActive = true;
    invoke("touch_down", x, y);
    for (let sampleIndex = 1; sampleIndex < touchDownSamples; sampleIndex += 1) {
      invoke("touch_move", x, y);
    }
  });
  listen(canvas, "pointermove", (event) => {
    if (event.buttons) {
      const [x, y] = point(event);
      invoke("touch_move", x, y);
    }
  });
  listen(canvas, "pointerup", releaseTouch);
  listen(canvas, "pointercancel", releaseTouch);
  listen(canvas, "lostpointercapture", releaseTouch);
  listen(window, "blur", releaseTouch);

  listen(boot, "pointerdown", (event) => {
    event.preventDefault();
    boot.setPointerCapture?.(event.pointerId);
    bootActive = true;
    invoke("boot_down");
  });
  listen(boot, "pointerup", releaseBoot);
  listen(boot, "pointercancel", releaseBoot);
  listen(boot, "lostpointercapture", releaseBoot);
  listen(window, "blur", releaseBoot);
  listen(document, "visibilitychange", () => {
    if (document.hidden) {
      releaseTouch();
      releaseBoot();
    }
  });

  return () => {
    releaseTouch();
    releaseBoot();
    for (const removeListener of listeners) {
      removeListener();
    }
  };
}

export function setupDemoUx(config) {
  const simulator = requireElement(".simulator", HTMLDivElement);
  const stage = requireElement(".stage", HTMLDivElement);
  const canvas = requireElement("#screen", HTMLCanvasElement);
  const body = document.body;

  body.classList.add("demo-ux-page", `demo-ux-page--${config.orientation}`);
  const galleryTag = buildGalleryTag(config.galleryUrl);
  const sceneCard = buildSceneCard(config);
  const deviceMode = buildDeviceMode({
    body,
    boot: requireElement("#boot-button", HTMLButtonElement),
    canvas,
    config,
    simulator,
    stage,
  });
  const notice = buildSimulatorNotice();

  body.append(
    galleryTag.link,
    sceneCard.restingButton,
    sceneCard.scrim,
    deviceMode.button,
    deviceMode.overlay,
    notice.element,
  );

  let timeControlBuilt = false;
  const showTimeControl = () => {
    if (timeControlBuilt) return;
    timeControlBuilt = true;
    body.classList.add("demo-ux-page--has-time-setter");
    buildTimeSetter({
      body,
      setTimeOfDay: (secondsOfDay) => secondsOfDay === -1
        ? config.handle.use_live_clock()
        : config.handle.set_clock_time_of_day(secondsOfDay),
      openDeviceMode: () => deviceMode.isActive(),
    });
  };

  let userZoom = 1;
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
    const fitScale = Math.min(1, availableWidth / naturalWidth, availableHeight / naturalHeight);
    simulator.style.transform = `scale(${fitScale * userZoom})`;
  };

  const zoomReset = document.createElement("button");
  zoomReset.type = "button";
  zoomReset.className = "demo-ux-zoom-reset";
  zoomReset.textContent = "reset zoom";
  zoomReset.hidden = true;
  zoomReset.addEventListener("click", () => {
    userZoom = 1;
    zoomReset.hidden = true;
    updateSceneScale();
  });
  simulator.addEventListener("wheel", (event) => {
    if (event.ctrlKey) {
      return;
    }
    event.preventDefault();
    userZoom = Math.min(2, Math.max(0.65, userZoom * Math.exp(-event.deltaY * 0.001)));
    zoomReset.hidden = userZoom === 1;
    updateSceneScale();
  }, { passive: false });
  document.body.append(zoomReset);

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

  return { showNotice: notice.show, showTimeControl };
}

function buildSimulatorNotice() {
  const element = document.createElement("div");
  element.className = "cyd-simulator-notice";
  element.hidden = true;
  element.setAttribute("role", "status");
  element.setAttribute("aria-live", "polite");
  element.setAttribute("aria-atomic", "true");

  let hideTimer = 0;

  const show = ({ severity = "info", message, durationMs = 2200 }) => {
    if (!["info", "warning", "fatal"].includes(severity)) {
      throw new Error(`unsupported simulator notice severity: ${severity}`);
    }
    if (typeof message !== "string" || message.length === 0) {
      throw new Error("simulator notice message must be non-empty");
    }
    if (!Number.isFinite(durationMs) || durationMs < 0) {
      throw new Error("simulator notice duration must be non-negative");
    }

    window.clearTimeout(hideTimer);
    element.className = `cyd-simulator-notice cyd-simulator-notice--${severity}`;
    element.textContent = message;
    element.setAttribute("role", severity === "fatal" ? "alert" : "status");
    element.hidden = false;
    if (durationMs > 0) {
      hideTimer = window.setTimeout(() => {
        element.hidden = true;
      }, durationMs);
    }
  };

  return { element, show };
}

function buildGalleryTag(galleryUrl) {
  const link = document.createElement("a");
  link.className = "demo-ux-gallery-tag";
  link.href = galleryUrl ?? "../../";
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
      <p>${escapeHtml(config.description)}</p>
    </section>
    <section class="demo-ux-card-section">
      <h3>Controls</h3>
      <p>${escapeHtml(config.controls)}</p>
    </section>
    <section class="demo-ux-card-section">
      <h3>Links</h3>
      <div class="demo-ux-card-links">
        <a href="${escapeHtml(config.coreCodeUrl)}" target="_blank" rel="noopener">Core code</a>
        <a href="${config.galleryUrl ?? "../../"}">Gallery</a>
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

function buildDeviceMode({ body, boot, canvas, config, simulator, stage }) {
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
  let bootPlaceholder = null;

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
    canvas.style.transform = canvas.dataset.inverted === "true" ? "rotate(180deg)" : "";
  };

  const canvasSizeObserver = new MutationObserver(resizeDeviceCanvas);
  canvasSizeObserver.observe(canvas, {
    attributes: true,
    attributeFilter: ["width", "height"],
  });

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
    canvas.style.transform = "";
  };

  const restoreBoot = () => {
    if (bootPlaceholder?.parentNode) {
      bootPlaceholder.replaceWith(boot);
      bootPlaceholder = null;
    }
    boot.style.position = "";
    boot.style.left = "";
    boot.style.top = "";
    boot.style.bottom = "";
    boot.style.transform = "";
  };

  const finishDeactivate = () => {
    restoreCanvas();
    restoreBoot();
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
    bootPlaceholder = document.createComment("demo-ux-boot-placeholder");
    boot.parentNode?.insertBefore(bootPlaceholder, boot);
    screenHost.append(canvas);
    overlay.append(boot);
    boot.style.position = "fixed";
    boot.style.left = "50%";
    boot.style.top = "auto";
    boot.style.bottom = "24px";
    boot.style.transform = "translateX(-50%)";
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
