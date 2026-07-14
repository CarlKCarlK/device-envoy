<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# Shared CYD WASM Simulator

## Status

Implementation in progress as of commit `57d533d`. The current work covers the canonical shell,
Linkage Blaze v4 migration, shared input lifecycle, simulator notices, and an
opt-in DNS Tester Wi-Fi simulation. The remaining work and evidence are
tracked below. This document does not authorize publishing or replacing
historical released pages by itself.

## Implementation work log

This section records what has been implemented so that the specification is
also a reviewable handoff rather than only a list of intentions.

### Completed work

- [x] Extracted the canonical browser shell into
  `crates/device-envoy-core/www/cyd-simulator.js` and
  `crates/device-envoy-core/www/cyd-simulator.css`, with canonical `case.png`
  and `desk.jpg` assets.
- [x] Migrated the current Linkage Blaze v4 Ballet, Clock, Skeleton Clock,
  and Armatron pages to `mountCydSimulator` and declarative application
  metadata. Historical v1-v3 pages remain self-contained.
- [x] Added Linkage Blaze `build-pages` copying and `check-cyd-shell`
  consistency verification for generated simulator assets.
- [x] Added shared Rust simulator construction, orientation-aware touch
  mapping, persistent flash support, physical BOOT input, calibration support,
  and exact inverted-orientation reporting.
- [x] Hardened browser input release handling for pointer cancel, lost pointer
  capture, window blur, visibility changes, and duplicate release events.
  Replaced WASM input listeners are removed before a new handle is bound.
- [x] Added the shared notice UI with informational, warning, and fatal
  presentation states, replacement, timeout, and live accessibility roles.
  DNS Tester's browser Wi-Fi message now uses this facility.
- [x] Added an opt-in shared WASM Wi-Fi simulation with typed captive-portal,
  connecting, and connection-reset outcomes. DNS Tester opts in; the four
  Linkage Blaze examples do not.
- [x] Regenerated DNS Tester v1 simulator assets and WASM output from the
  canonical sources with cache-busted references.
- [x] Verified Device Envoy core/WASM checks, DNS Tester WASM compilation,
  Linkage Blaze shell consistency, and the Linkage Blaze Playwright suite.
  The suite now starts a repository-owned server on a dynamically selected
  port; the full run passed all ten tests.
- [x] Added a real DNS Tester Playwright integration test covering startup
  notices, simulated Wi-Fi, BOOT interruption and restart, calibration,
  orientation, reload persistence, and browser error reporting.
- [x] Fixed DNS Tester's startup lifecycle so orientation queries and input
  events remain usable while asynchronous calibration and Wi-Fi startup are
  in progress. The live simulator control is kept outside the mutable app
  state borrow.
- [x] Added typed Rust-side simulator notice requests with stable identifiers,
  warning/info/fatal severity, replacement through a single pending queue, and
  fatal-loop disposition. The DNS Tester browser shell now consumes those
  identifiers instead of inventing notice severity from JavaScript.

### Remaining work

- [x] Decide and document the `ConnectionFailed` simulation policy. The normal
  deterministic simulation emits captive-portal-ready and connecting before
  success; `ConnectionFailed` is reserved for a future explicit failure
  injection path and is never emitted spontaneously.
- [ ] Update this specification and
  `CYD_WASM_CONSTRUCTION_MEDIUM_ARTICLE.md` as each acceptance area is
  verified, then remove obsolete planning text rather than leaving completed
  work described as future work.
- [x] Run the full Linkage Blaze `just check-all` after the browser harness and
  DNS integration test were added; all checks and embedded example builds
  passed.
- [x] Identify and run the Device Envoy full validation command as
  `cargo check-all`; it passes after refreshing the generated ESP examples.
  Targeted core, DNS WASM, formatting, and JavaScript checks also pass.

The simulator is not complete until the remaining checklist items are either
implemented and tested or explicitly removed from the acceptance criteria by
the project owner.

## Objective

Turn the existing WASM CYD support into one complete, reusable browser CYD
platform that serves all five current CYD examples:

- Linkage Blaze Armatron;
- Linkage Blaze Ballet;
- Linkage Blaze Clock;
- Linkage Blaze Skeleton Clock;
- Device Envoy DNS Tester.

The simulator must own both sides of the virtual device boundary:

1. Rust device semantics such as display orientation, calibrated touch,
   physical-button state, flash persistence, reset, and animation scheduling.
2. Browser presentation such as the desk background, device case, screen
   frame, cord, BOOT button, responsive sizing, wheel zoom, full-screen mode,
   gallery link, and “tap for details” card.

The code that personalizes the shared simulator for one application should be
as direct and comprehensible as the constructor code in an ESP example. An
application must not implement its own virtual board, DOM event bridge, canvas
orientation policy, reset lifecycle, or copy of the common web presentation.

## Success measure

Adding a sixth CYD example should require:

- one small Rust WASM launcher that constructs the shared simulator resources
  and calls the existing platform-neutral application entry point;
- one declarative web configuration containing application metadata and
  explicitly requested optional controls;
- application-specific assets only;
- application-specific browser services only when the browser truly needs a
  substitute for hardware functionality.

It should not require copying or editing the simulator HTML structure, shared
CSS, pointer handling, BOOT handling, full-screen implementation, orientation
code, case alignment, desk background, or details-card implementation.

## Current evidence

The current pages are already one design implemented repeatedly:

- the four Linkage Blaze v3 demos each contain an identical 419-line
  `demo-ux.js`;
- the four Linkage Blaze v3 demos each contain an identical 342-line
  `demo-ux.css`;
- DNS Tester contains another copy that has already started to diverge;
- all five pages use byte-identical `case.png` assets;
- all five pages use byte-identical `desk.jpg` assets;
- landscape and portrait pages repeat the same case, cord, canvas, and BOOT
  layout constants in page-local CSS;
- fixed-orientation examples have mostly inert browser device controls, while
  DNS Tester independently implements touch, reset, flash, and orientation;
- full-screen sizing and normal-stage sizing are currently separate code paths
  that individual pages can accidentally update at different times.

This duplication is not application customization. It is an unextracted WASM
platform.

## Design principles

### Model a board, not a particular application

The shared simulator represents a CYD and its browser host. It must not know
about DNS counters, robot-arm joints, ballet frames, or clock hands.

Application code remains responsible for application state and rendering.
Simulator code remains responsible for device input, persistence, reset,
orientation, and presentation.

### Use ESP behavior as the reference

When browser behavior is ambiguous, match the observable ESP CYD behavior:

- orientation is selected before a frame is presented;
- reset clears transient input state;
- flash survives reset;
- a held physical button cannot become an endless sequence of synthetic reset
  requests;
- calibration uses fixed landscape-panel coordinates;
- display orientation and touch orientation are applied exactly once;
- application startup sees a coherent set of device resources;
- a Wi-Fi-using application's connect phase fires the same connect events and
  accepts the same BOOT interruption as ESP, not a silent skip to success.

### Keep explicit application construction

Do not replace straightforward application setup with a large builder or a
single trait containing every optional browser feature. Prefer direct
constructors, resource structs, and a small standard simulator protocol.

### One source, many deployed copies

The canonical simulator source must exist once. Build tooling may copy or
bundle versioned simulator files into each repository’s published pages so old
URLs remain self-contained. Checked-in historical web releases must never
depend at runtime on another repository’s current branch or Pages deployment.

### Dynamic orientation is normal

The simulator must treat all four orientations as ordinary device states. A
fixed-orientation example is merely an application that never requests a
change, not a separate simulator implementation.

## Non-goals

- Do not extract ESP and RP application startup into a generic lifecycle
  framework solely to resemble WASM.
- Do not move application rendering or application policy into JavaScript.
- Do not change the visual output inside the CYD screen except where a failing
  parity test proves the existing output is wrong.
- Do not include the Linkage Blaze editor or the separate Three.js Armatron
  viewer in the five-app simulator migration.
- Do not make released historical page versions load mutable shared resources
  over the network.
- Do not make every optional application control visible on every example.
- Do not use square test canvases or center-only touch tests that conceal
  orientation errors.

## Target architecture

```text
Application-neutral Rust core
  ballet / clock / skeleton_clock / armatron / dns_tester
                              |
                              v
                         Cyd trait
                              |
                              v
                  shared CydSimulatorWasm
          display, touch, button, flash, reset, time
                              |
                              v
                   standard WASM app handle
                              |
                              v
                   shared browser CYD shell
        DOM, frame, case, cord, zoom, details, full screen
```

The architecture has three deliberate layers.

### Layer 1: `CydWasm` device semantics

Strengthen the existing Device Envoy WASM implementation rather than creating
app-specific device wrappers. This layer owns:

- `CydDisplayWasm` and exact orientation retention;
- calibrated `CydTouchWasm` events in fixed landscape-panel coordinates;
- `ButtonWasmSource` and reset-safe button transitions;
- `FlashBlockWasm` namespaces and persistence behavior;
- animation-frame scheduling and browser delays;
- transient-state reset behavior;
- browser substitutes for device services when those substitutes are generic.

This layer must not query page-specific DOM outside the canvas and standard
input sources.

### Layer 2: standard WASM simulator runtime

Add the smallest reusable runtime to `device-envoy-core` behind its existing
`wasm` feature. It constructs and supervises a virtual CYD application. The
final names may differ, but the application-facing code should have this
shape:

```rust,no_run
# use device_envoy_core::cyd::display::Orientation;
# async fn example() -> Result<(), wasm_bindgen::JsValue> {
# let canvas = todo!();
let simulator = CydSimulatorWasm::new(canvas, Orientation::Landscape)?;
let (mut cyd, mut button, simulator_control) = simulator.into_parts();

wasm_bindgen_futures::spawn_local(async move {
    application_core(&mut cyd, &mut button).await;
});

simulator_control
# }
```

This is an architectural example, not a mandate for these exact names or an
infallible application return type.

The runtime must provide a standard browser-facing handle or protocol for:

- starting and restarting the application;
- pointer down, move, up, and cancel;
- BOOT down, up, cancel, and lost-focus release;
- current orientation and inversion;
- simulator notices and host requests;
- resetting transient input state;
- observing intrinsic canvas-size changes;
- clearing development storage when explicitly requested.

Every app-specific WASM crate should use this protocol by construction from
the shared `device-envoy-core` pieces, not by copying its implementation.

### Layer 3: canonical browser shell

Provide one canonical JavaScript module, stylesheet, HTML template or mounting
function, and asset set. A page should personalize it declaratively:

```js
await mountCydSimulator({
  wasm: { init, start },
  app: {
    title: "DNS Tester",
    initialOrientation: "landscape",
    preview: "Exercise CYD touch, orientation, and reset behavior.",
    descriptionHtml: "<p>...</p>",
    controlsHtml: "<p>...</p>",
    coreCodeUrl: "https://github.com/...",
    galleryUrl: "../../",
  },
});
```

Optional capabilities such as a clock time setter or development alignment
controls should be explicit optional values. Absence must mean absence; the
page must not provide inert controls.

Application bootstrap code must not directly:

- construct the case, cord, frame, screen, BOOT button, details card, gallery
  link, or full-screen overlay;
- attach pointer, wheel, resize, mutation, BOOT, or full-screen listeners;
- calculate canvas coordinates or orientation transforms;
- manipulate canvas width, height, inversion, or CSS transforms;
- implement generic toast timing;
- copy shared CSS constants.

## Standard browser application protocol

Define and document one stable protocol between the browser shell and each
application WASM module. It may be represented by a `wasm-bindgen` class or by
functions plus an opaque handle, but all five apps must expose the same core
operations.

At minimum the protocol must cover:

- `start` or equivalent construction;
- pointer down, move, up, and cancel;
- BOOT down, up, and cancel;
- current orientation and inversion;
- application or simulator restart requests;
- host notices;
- fatal runtime errors;
- optional application extensions registered separately from the core
  protocol.

Do not encode host events as undocumented strings. Use a Rust enum internally
and generate or centrally define the JavaScript representation. Exhaustive
matching must make a newly added event visible to both sides at compile time or
in a protocol test.

## Complete presentation contract

All five pages must share the same presentation behavior.

### Page and scene

- Use the same tiled `desk.jpg` background and scale.
- Present the same CYD case image, drop shadow, screen opening, and USB cord.
- Use one `.simulator`, `.stage`, screen, case, cord, and BOOT structure.
- Keep the canvas pixelated and aligned exactly with the case opening.
- Derive stage dimensions from the intrinsic canvas orientation.
- Support landscape, portrait, landscape-inverted, and portrait-inverted
  without page-specific CSS.
- Apply inversion to the complete device presentation consistently; do not
  rotate touch a second time.
- Keep the physical frame and screen aligned during orientation changes,
  startup splashes, calibration, reset, browser resize, and full-screen entry.

### Responsive fit and wheel zoom

The simulator must combine automatic responsive fitting with explicit user
zoom.

- Compute a base scale that fits the complete device scene in the viewport.
- Maintain a separate user zoom multiplier so resize does not erase the
  user’s selected zoom.
- Wheel input over the simulator adjusts the user zoom smoothly and calls
  `preventDefault` only while it is controlling simulator zoom.
- Clamp zoom to documented useful limits rather than permitting zero,
  negative, or unbounded scales.
- Preserve the device’s aspect ratio at every zoom level.
- Provide an accessible way to reset zoom, such as a documented keyboard
  shortcut, double-click, or small reset action in the details controls.
- Trackpad and mouse-wheel deltas must be normalized so one device does not
  jump from minimum to maximum zoom.
- Full-screen device mode uses fit-to-viewport sizing and must not inherit a
  stale scene zoom transform.

Wheel zoom belongs to the simulator frame. If an application genuinely needs
wheel input inside its CYD screen, the protocol must provide an explicit
opt-out or routing policy rather than relying on event-listener order.

### Gallery control

- Show the same visible gallery/back control in all five pages.
- Make the target URL configurable because Device Envoy and Linkage Blaze have
  different gallery roots.
- Preserve the current paper-tag visual language and accessible label.
- Do not hard-code `../../` in the shared module.

### “Tap for details” card

- Show the same resting card, preview line, “tap for details” hint, scrim,
  dialog, close control, sections, and source-code link.
- Keep title, preview, description, controls, and source URL declarative.
- Escape plain metadata and accept trusted application HTML only in fields
  explicitly documented as HTML.
- Preserve keyboard focus, Escape behavior, and dialog accessibility.

### Full-screen device mode

- Provide the same full-screen action and close control for every app.
- Move the live canvas rather than cloning or replacing it.
- Size from current intrinsic canvas width and height every time they change.
- Preserve portrait/landscape aspect ratio and inverted orientation.
- Recalculate during orientation changes, resize, full-screen transitions, and
  mobile browser viewport changes.
- Restore the canvas to the exact original placeholder when leaving.
- Do not lose pointer, BOOT, or application bindings while moving the canvas.
- Hide page-only controls consistently while device mode is active.

### BOOT control

- Display one consistently styled physical BOOT button in the framed scene.
- Place it relative to the case so it remains visible in both landscape and
  portrait layouts and at all supported zoom levels.
- Forward pointer down and release separately; do not reduce BOOT to a click.
- Treat pointer cancel, lost capture, window blur, visibility loss, and reset
  as release paths.
- Start every simulated reset with clean transient button state while
  preserving flash.
- Never replay one held browser pointer as repeated application resets.
- If an application does not use BOOT, either give it the shared platform
  meaning or explicitly hide it. Do not render an inert physical control.

### Touch and pointer input

- Use pointer capture for a press that begins on the CYD screen.
- Convert CSS coordinates to intrinsic canvas pixels with independent width
  and height scale factors.
- Apply full-screen scaling and inversion exactly once.
- Feed the documented fixed landscape-panel coordinate contract into
  `CydTouchWasm`.
- Handle down, move, up, cancel, lost capture, window blur, and visibility
  change.
- Prevent stale events from surviving reset or orientation reconstruction.
- Centralize any synthetic calibration sample generation; applications must
  not manufacture their own browser sampling policy.
- Preserve touch behavior while the canvas is moved into full-screen mode.

### Orientation

- Store exact orientation, including both inverted variants.
- Update Rust orientation, intrinsic canvas dimensions, stage dimensions,
  case/cord layout, inversion transform, and touch mapping as one transition.
- Apply the new orientation before drawing a restart splash or first
  application frame.
- Never infer inversion from width and height.
- Fixed-orientation applications use the same mechanism with orientation
  changes disabled by application policy.

### Frame pacing and startup

- Centralize `requestAnimationFrame` scheduling primitives.
- A startup splash delay must not delay application of the selected
  orientation or correct aspect ratio.
- Restart must cancel or supersede the previous application task so two loops
  cannot draw concurrently.
- One application exit or host notice must not silently leave the simulator
  unable to accept further input.
- Hidden tabs and throttled animation frames must not corrupt reset or input
  state.

### Flash and reset

- Give each application explicit, collision-free flash namespaces.
- Preserve flash across simulator restart and page reload.
- Clear transient input queues and interaction state on restart.
- Provide a development-only clear-storage operation through shared UI or a
  documented API.
- Test corrupted, missing, and valid records.
- Keep app-specific record schemas in application code while keeping storage
  mechanics in the simulator platform.

### Generic simulator notices

Provide one shared toast/bubble facility for browser-platform messages such as
unsupported Wi-Fi, storage failure, or simulated hardware limitations.

- The simulator owns placement, styling, duration, replacement, and
  accessibility announcement.
- Applications provide a typed notice request and text or a notice identifier.
- A notice must not terminate the application input loop unless its typed
  severity is fatal.
- Notices remain readable in normal and full-screen modes.

### Optional application controls

The canonical shell must support optional extensions without forks:

- Clock and Skeleton Clock time setters;
- Armatron development alignment controls when explicitly enabled;
- preview auto-boot for screenshot generation;
- simulated Wi-Fi connect for applications that use Wi-Fi on real hardware
  (see "Wi-Fi connect simulation" below);
- application-specific actions that do not pretend to be CYD hardware.

Extensions register through documented slots or callbacks. They must not copy
or modify the simulator’s core DOM and listeners.

### Wi-Fi connect simulation

An application that uses Wi-Fi on real hardware (currently DNS Tester) must
exercise the same connect-time event flow and BOOT interruption in the
browser rather than skip straight to a fake success or a generic "Wi-Fi
unsupported" notice.

- Provide this as a shared, platform-level simulator alongside
  `CydSimulatorWasm`, not as an application-specific fake. Only applications
  that use Wi-Fi on real hardware register it; it must remain entirely
  absent, not merely inert, for applications that have no Wi-Fi on real
  hardware. None of the four current Linkage Blaze examples use Wi-Fi.
- Fire the same normal connect-event values that the ESP launcher's Wi-Fi
  connect call fires (captive-portal-ready and connecting), so application
  event handlers require no WASM-specific branching. Keep `ConnectionFailed`
  available for explicit failure injection, but do not invent nondeterministic
  failures in the normal browser path.
- Simulate the connect delay with a fixed wait of a few seconds, then report
  success. Never attempt real network activity from the browser.
- Race the simulated wait against BOOT exactly as the ESP connect call does:
  a BOOT press during the wait clears simulated Wi-Fi state and restarts the
  application, matching the reset-to-captive-portal-then-reboot behavior on
  ESP (a WASM restart, not a real reboot).

## Canonical source location

The canonical implementation lives in the Device Envoy repository. It is a
platform surface in the existing core package, not part of the DNS Tester
application:

```text
device-envoy/
  crates/device-envoy-core/src/wasm.rs
      low-level CydWasm display, touch, button, flash, and frame primitives
  crates/device-envoy-core/src/wasm/simulator.rs
      reusable simulator runtime and wasm-bindgen protocol
  crates/device-envoy-core/www/
      cyd-simulator.js
      cyd-simulator.css
      case.png
      desk.jpg
```

`device-envoy-core/src/wasm.rs` remains the implementation of the `Cyd` device
traits and, behind the existing `wasm` feature, also owns the reusable
browser-device protocol, reset supervision, and simulator runtime. It must not
depend on DNS Tester or any Linkage Blaze application.

The `device-envoy-core/www/` files are the canonical browser shell. They
contain the common frame, case, cord, desk, gallery, details, full-screen,
zoom, input, and BOOT presentation. They are not copied from one of the five
applications.

The canonical source owns:

- simulator JavaScript;
- simulator CSS;
- page/template fragments or mounting code;
- `case.png`;
- `desk.jpg`;
- browser tests and fixtures.

The core package must make its `www/` files available so Linkage Blaze build
tooling can consume a released simulator version without depending on Device
Envoy’s working tree. During local cross-repository development, the Linkage
Blaze workspace may use a path dependency or sibling checkout, just as it
already does for Device Envoy Rust crates.

Do not hide canonical assets inside one example’s versioned output directory.

## Repository boundary and deployed copies

The source ownership and deployment ownership are intentionally different:

```text
canonical implementation
    device-envoy/crates/device-envoy-core/
              src/wasm.rs (behind the `wasm` feature)
              www/
              |
              +--> Device Envoy DNS Tester versioned Pages output
              |
              +--> Linkage Blaze versioned demo Pages output
```

Each published page receives a self-contained copy of the simulator web
assets and generated application WASM package. A historical page must not load
`cyd-simulator.js`, CSS, images, or WASM from another repository’s mutable
`main` branch or current Pages deployment.

The five application launchers remain in their owning repositories:

```text
device-envoy/
  crates/device-envoy-dns-tester-wasm/

linkage-blaze/
  application-specific WASM crates and page launchers
```

Those launchers may depend on the released `device-envoy-core` WASM surface and
register their application core, metadata, and optional capabilities. They
must not become a second home for simulator behavior.

Linkage Blaze and Device Envoy build tools must copy or bundle those canonical
files into each newly built versioned page. Add a generated-file consistency
check so hand-edited copies fail CI.

Historical page versions remain frozen. Migrate current/new versions rather
than rewriting old releases unless the human explicitly chooses otherwise.

## Personalization boundary

After migration, application-specific web code may contain:

- WASM imports and application startup;
- title and metadata;
- description and control text;
- gallery and source-code URLs;
- initial or allowed orientations;
- optional time setter or development extension registration;
- genuine browser substitutes for application services.

It must not contain generic virtual-device behavior.

Application-specific Rust WASM code may contain:

- construction of application-specific services;
- direct construction of shared simulator resources;
- one call to the platform-neutral application entry point;
- mapping of application exits to typed simulator commands;
- unavoidable application error translation.

The result should read like an ESP launcher: construct resources, call the app,
handle a small typed exit. Avoid wrappers whose methods merely rename every
method on `CydWasm`, `ButtonWasmSource`, or `FlashBlockWasm`.

## Migration phases

### Phase 1: freeze behavior and build the test fixture

Before moving code, capture current screenshots and DOM behavior for all five
pages in normal and full-screen mode. Record intended differences rather than
assuming every current difference is a feature.

Create a tiny simulator fixture application that can:

- fill the screen with orientation-specific asymmetric markers;
- report pointer coordinates;
- request all four orientations;
- count BOOT transitions and resets;
- read and write flash;
- request informational and fatal notices.

Use this fixture for platform tests so failures are not confused with one of
the five applications.

### Phase 2: complete Rust device semantics

Strengthen `CydWasm` and related sources until the fixture passes orientation,
touch, reset, button, flash, and scheduling tests without page-specific code.

Remove coordinate and reset policy from DNS Tester’s browser wrapper as those
capabilities become simulator responsibilities.

### Phase 3: create the canonical browser shell

Consolidate the identical Linkage Blaze `demo-ux.js`, `demo-ux.css`, page
structure, case alignment, desk background, cord, BOOT styling, and common
assets.

Add dynamic orientation, wheel zoom, robust full-screen sizing, typed notices,
and complete input cleanup to the canonical implementation.

### Phase 4: migrate fixed-orientation display examples

Migrate Ballet, Clock, and Skeleton Clock first. They exercise portrait and
landscape display, frame pacing, details, full-screen, gallery, time setters,
and responsive presentation with little touch/reset complexity.

Delete their current duplicate simulator JS/CSS from the source path after
generated copies are proven reproducible.

### Phase 5: migrate Armatron

Migrate Armatron to exercise CYD pointer input, BOOT, optional development
controls, preview auto-boot, PWA/service-worker integration, and landscape
presentation.

Do not mix the separate Three.js viewer’s wheel behavior into CYD simulator
wheel zoom.

### Phase 6: migrate DNS Tester

Migrate DNS Tester last because it exercises the full platform:

- all four orientations;
- calibration;
- persistent flash;
- simulated reset;
- BOOT recovery;
- startup splash;
- deterministic DNS;
- unsupported Wi-Fi notice;
- normal and full-screen dynamic aspect ratio.

Remove generic browser-device code from `DnsTesterWeb` and `main.js`. Retain
only DNS-specific service substitution, application metadata, and typed exit
mapping that cannot live in the simulator.

### Phase 7: enforce one source

Delete obsolete source copies and add CI checks that:

- current pages use the canonical simulator package;
- generated assets match canonical sources;
- no current app reintroduces forbidden generic listeners or copied shell
  styles;
- versioned outputs contain all resources needed for independent deployment.

## Test strategy

### Rust host and WASM tests

Test device semantics independently of the five apps:

- all four exact orientations round-trip;
- every representative and exhaustive touch point maps correctly;
- reset clears touch queues and button state but preserves flash;
- a held BOOT press produces at most one reset request;
- pointer cancel and lost focus release input;
- orientation is applied before the first post-reset frame;
- only one application task owns the display after restart;
- flash namespaces do not collide;
- host notices do not stop input unless fatal.

### Page-level Playwright tests

Use the existing Linkage Blaze Playwright dependency or an equivalent
repository-owned browser harness. Tests must load the real HTML, JavaScript,
generated WASM, CSS, and assets—not just instantiate a canvas from a Rust WASM
test.

Test the fixture and all five pages for:

- successful startup with no console errors;
- expected gallery link and details card;
- details open, close, focus, and Escape behavior;
- desk background, case, cord, frame, canvas, and BOOT visibility;
- normal-mode aspect ratio;
- full-screen aspect ratio and restoration;
- wheel zoom, clamp, reset, and resize interaction;
- BOOT down/up/cancel and one-reset behavior;
- touch down/move/up/cancel routing;
- page reload and flash persistence;
- cache-busted loading of JavaScript and WASM artifacts;
- no duplicate event handlers after restart.

For DNS Tester, additionally test every orientation in normal and full-screen
mode, including the splash before the dashboard. Assert element and canvas
dimensions during the splash rather than only after startup completes.

### Visual regression tests

Capture reviewed screenshots for each app in its default scene and full-screen
mode. Also capture the simulator fixture in landscape and portrait.

Visual tests must check the complete browser presentation, not only the CYD
framebuffer. Retain existing framebuffer goldens because browser screenshots
and framebuffer goldens prove different boundaries.

Use tolerances or deterministic browser settings deliberately. Do not accept
large blanket screenshot thresholds that would hide a stretched canvas or
misaligned case.

### App integration tests

Each application needs only focused smoke tests proving that it uses the
standard protocol and its optional extensions work:

- Ballet renders and advances;
- Clock renders and its time setter works;
- Skeleton Clock renders and its time setter works;
- Armatron receives touch and BOOT;
- DNS Tester rotates, calibrates, resets, persists, and continues after its
  unavailable-Wi-Fi notice.

Do not duplicate the full simulator contract suite in every application.

## Build, versioning, and CI

Provide repeatable commands that:

- build each application WASM module;
- bundle or copy the canonical simulator shell and assets;
- generate cache-busted JavaScript and `.wasm` references from content or a
  build version rather than manually incremented literals;
- serve both repositories locally over HTTP;
- run the shared Playwright suite;
- verify generated files;
- build all five current pages.

Integrate the relevant checks into each repository’s local CI. Device Envoy’s
completed work must pass `just check-all`; Linkage Blaze’s completed work must
pass its corresponding full local CI command.

Do not patch only generated page files. Change canonical sources first and
regenerate outputs.

## Acceptance criteria

The work is complete when:

- one canonical simulator implementation serves all five current CYD apps;
- all five pages use the same desk, frame, case, cord, BOOT, gallery, details,
  full-screen, responsive-fit, and wheel-zoom behavior;
- dynamic and fixed orientation use one implementation;
- all four orientations render at the correct aspect ratio before splash and
  application frames;
- normal and full-screen touch use one coordinate pipeline;
- reset starts with clean transient input and preserved flash;
- BOOT cannot cause an infinite simulated-reset loop;
- DNS Tester's simulated Wi-Fi connect fires the same connect events and
  accepts the same BOOT-triggered reset as ESP, without real network
  activity, and no non-Wi-Fi application carries this simulation;
- simulator notices do not accidentally terminate otherwise recoverable apps;
- no current app owns generic canvas sizing, pointer forwarding, orientation,
  full-screen, wheel zoom, or frame DOM code;
- no current app maintains a hand-edited copy of common simulator JS or CSS;
- app-specific Rust launchers read like direct platform constructors followed
  by one core application call;
- app-specific JavaScript is declarative metadata plus genuine optional app
  extensions;
- one shared page-level browser suite covers the simulator contract;
- each app has a small focused integration test;
- complete-page visual regressions cover default and full-screen presentation;
- generated deployment artifacts are reproducible and cache-safe;
- historical released URLs remain self-contained and unchanged unless
  explicitly migrated.

## Completion review

Before declaring the simulator complete, add a sixth minimal example in a test
or temporary fixture using only the public construction path. Review its Rust,
JavaScript, HTML, CSS, and assets.

If that example must know how to align the case, map a pointer, release BOOT,
resize full-screen canvas, apply orientation, copy common CSS, or implement
details/gallery UI, the abstraction is not finished.
