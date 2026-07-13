<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# Example Crate Reorganization

## Status

Planning only. This document describes the intended organization for the DNS
Tester and Conway applications. It does not authorize implementation by itself.

## Purpose

Reorganize Device Envoy's substantial CYD examples around the same boundary
used by Linkage Blaze:

- one `no_std`, platform-neutral crate owns reusable application code;
- ESP and RP example crates contain thin hardware launchers;
- browser crates contain thin `wasm-bindgen` and JavaScript adapters;
- `CydMemory` executes the same application code used on hardware and produces
  the golden previews.

The immediate applications are DNS Tester and Conway. The organization should
also provide an obvious home for future applications that are more than short
API demonstrations.

## Current problems

DNS Tester currently shares its bitmap renderer, but not the complete
application. ESP and WASM separately implement state, touch dispatch,
orientation actions, calibration restart behavior, Wi-Fi messaging, DNS test
completion, and redraw scheduling. Fixes therefore drift between platforms.

Conway already has a dedicated core crate, but it is a one-off organization
rather than part of a consistent examples architecture. Its platform launchers
and browser packaging should follow the same boundaries as DNS Tester.

Application code also currently lives in `device-envoy-core`, for example the
DNS Tester renderer. `device-envoy-core` should provide reusable device
abstractions, display plumbing, storage traits, and platform-neutral drivers;
it should not become a collection of complete applications.

## Linkage Blaze model

Linkage Blaze uses these principal crates:

```text
linkage-blaze-core/
  src/examples/
    armattron/
    ballet.rs
    clock.rs
    skeleton_clock.rs

linkage-blaze-examples-esp/
linkage-blaze-examples-rp/
```

The core example modules own application behavior and render against generic
Device Envoy interfaces. ESP and RP examples primarily construct devices and
call the shared entry points.

Device Envoy should use the same architectural boundary. It does not need to
copy every LB packaging detail where the browser deployment model differs.

## Target workspace organization

```text
crates/
  device-envoy-core/
    src/
      cyd.rs
      memory.rs
      wasm.rs
      wifi_auto.rs
      ...device abstractions only...

  device-envoy-examples-core/
    build.rs
    src/
      lib.rs
      dns_tester.rs
      dns_tester/ui.rs
      conway.rs
    docs/assets/
      dns_tester/
    tests/
      dns_tester_memory.rs
      conway_memory.rs
    tests/assets/
      dns_tester_landscape.png
      dns_tester_portrait.png
      dns_tester_landscape_inverted.png
      dns_tester_portrait_inverted.png
      dns_tester_splash.png
      conway.png

  device-envoy-examples-esp/
    examples/
      ...generated board launchers...
    examples/templates/
      dns_tester.rs.j2
      conway.rs.j2

  device-envoy-examples-rp/
    examples/
      dns_tester.rs
      conway.rs

  device-envoy-dns-tester-wasm/
  device-envoy-conway-wasm/
```

The two WASM crates remain separate leaf crates. They produce independent
versioned web artifacts and should not force both applications into every WASM
binary. Each should depend on `device-envoy-examples-core` and contain only the
browser adapter for its application.

If moving all ESP/RP examples into new `device-envoy-examples-esp` and
`device-envoy-examples-rp` crates would create an unnecessarily large first
change, the migration may temporarily leave launchers under
`device-envoy-esp/examples` and `device-envoy-rp/examples`. The shared core
boundary is required in the first phase; the final platform crate layout above
remains the goal.

## Dependency direction

Dependencies must point in one direction:

```text
device-envoy-core
        ↑
device-envoy-examples-core
        ↑
  ┌─────┼───────────┬───────────────────────────┐
  │     │           │                           │
 ESP    RP    DNS Tester WASM              Conway WASM
```

Rules:

- `device-envoy-core` must not depend on an examples crate.
- `device-envoy-examples-core` must remain `no_std` and allocation-free unless
  a separately reviewed requirement changes that goal.
- Shared application code must not use ESP, RP, `web-sys`, `wasm-bindgen`, or
  browser APIs.
- Platform crates may depend on both `device-envoy-core` and
  `device-envoy-examples-core`.
- Shared assets belong to `device-envoy-examples-core`; versioned web-page
  furniture such as the desk and CYD case remains under `docs/`.

## Shared application boundary

Each application should expose a state type and a small set of explicit inputs
and outputs. Platform launchers should not reproduce its state machine.

### DNS Tester

The shared DNS Tester module should own:

- query, success, and failure counts;
- last latency and last result;
- test/dashboard, Wi-Fi connecting, Wi-Fi setup, and unavailable states;
- control layout and hit testing for all orientations;
- CAL, WI-FI, ROTATE, and ordinary test-tap actions;
- selection of dynamic labels, values, colors, and fonts;
- rendering the static TGA backgrounds and dynamic values;
- transitions produced by DNS success, empty responses, and errors;
- the platform-neutral portion of calibration and orientation restart flow.

The shared API should make platform differences explicit. A likely shape is:

```rust,no_run
pub struct DnsTesterApp {
    // Platform-neutral state.
}

pub enum DnsTesterInput {
    Touch(TouchEvent),
    WifiConnecting,
    WifiSetup,
    WifiReady,
    DnsFinished(DnsResult),
}

pub enum DnsTesterAction {
    None,
    StartDnsLookup,
    ClearCalibrationAndRestart,
    ResetWifiAndRestart,
    SaveOrientationAndRestart(Orientation),
}
```

Names may change during implementation, but the data flow should remain
explicit: platform code reports events, the shared app returns actions, and the
shared app renders its resulting state.

DNS resolution itself is platform-provided:

- ESP and RP perform a real `embassy-net` DNS query and report its result and
  measured latency.
- WASM reports a deterministic simulated result because browser JavaScript
  cannot issue arbitrary DNS requests.
- `CydMemory` tests inject success and failure outcomes directly.

The core application must not know about `embassy-net::Stack`, JavaScript
Promises, or a particular Wi-Fi implementation.

### Conway

The shared Conway module should own:

- board storage and generation stepping;
- timing-independent simulation state;
- touch/button actions and any orientation behavior;
- drawing through generic Device Envoy display interfaces;
- deterministic initialization used by hardware, WASM, and memory tests.

Platform launchers should supply clocks, display/touch devices, persistence if
needed, and event-loop scheduling. There must be one simulation and rendering
implementation, not a hardware version and a browser version.

## Runner model

The examples-core crate should support both continuously running hardware and
browser-driven stepping without duplicating application logic.

- Application state transitions and rendering are shared methods.
- ESP/RP launchers may call a shared async `run` function when the platform can
  yield forever inside an Embassy task.
- WASM may call the same transition/render methods from browser callbacks or
  animation frames.
- A shared forever-running helper must be built from the same public state
  methods; it must not become a second implementation.

This distinction is necessary because a browser event loop and an MCU Embassy
task have different ownership and scheduling requirements, even when the
application behavior is identical.

## Platform launcher responsibilities

ESP and RP launchers should be limited to:

- board pin/resource construction;
- static workspace allocation;
- creating the CYD display and touch implementations;
- creating flash blocks;
- invoking Wi-Fi setup and providing a DNS resolver;
- measuring operation duration with the platform clock;
- translating shared actions that require reset or platform services;
- calling the shared application transition/render API.

WASM wrappers should be limited to:

- constructing `CydWasm`, `FlashBlockWasm`, and browser button/touch sources;
- exposing the minimum `wasm-bindgen` API used by JavaScript;
- forwarding browser inputs to the shared application;
- executing shared actions such as persistence and reinitialization;
- adapting the deterministic DNS result;
- synchronizing the decorative browser shell with shared orientation state.

The wrappers must not contain copies of counter updates, control rectangles,
orientation cycles, screen-state decisions, or rendering coordinates.

## Assets and previews

Move the authoritative DNS Tester design bundle to:

```text
crates/device-envoy-examples-core/docs/assets/dns_tester/
```

The SVGs remain the editable source. Production TGA files contain static
artwork only. Dynamic slot coordinates and touch regions must have one
machine-readable or Rust source of truth used by the application; Markdown and
SVG comments may document those values but must not become independent copies
that drift.

The examples-core crate must generate the production TGA backgrounds from the
SVG sources in its Rust `build.rs`. The build must:

- use Rust build dependencies rather than requiring Python, FFmpeg, or another
  separately installed executable;
- omit sample dynamic values and developer-only guides identified by the SVG
  design metadata;
- rasterize exact 320x240 landscape and 240x320 portrait images;
- emit deterministic, uncompressed true-color TGA files into `OUT_DIR`;
- emit `cargo:rerun-if-changed` directives for every source asset and layout
  metadata file;
- fail with a useful error when an SVG is malformed, a required layer is
  missing, or an output has the wrong dimensions; and
- make the generated bytes available to the shared renderer with
  `include_bytes!(concat!(env!("OUT_DIR"), ...))` or an equivalent single
  compile-time inclusion path.

Generated TGA files are build artifacts, not a second editable source of truth.
They should not be checked in unless a later release process establishes a
specific need for distributable generated assets. Checked-in golden PNGs remain
the human-reviewable record of expected rendered output.

TODO0ARTICLE The SVG-to-TGA conversion is not a `const fn`. This shows where
`build.rs` is still best.

`CydMemory` tests belong to `device-envoy-examples-core` and must:

- render the same application state used by ESP/RP/WASM;
- drive the public application input/step/render API rather than a test-only
  renderer;
- produce distinct golden frames for landscape, portrait, landscape inverted,
  and portrait inverted;
- reach those four frames by activating ROTATE repeatedly, proving both the
  orientation sequence and its persistence path;
- verify the two inverted frames are sensible 180-degree presentations, not
  copies of their non-inverted counterparts;
- exercise each control hitbox after every orientation change;
- cover the splash, Wi-Fi connecting/setup, ready, DNS success, DNS failure,
  and browser-unavailable states;
- verify calibration temporarily uses landscape coordinates and then restores
  the previously selected application orientation;
- use scripted touch, button, flash, and DNS outcomes wherever `CydMemory`
  provides a sensible platform substitute;
- produce the canonical Conway preview;
- fail when rendered output changes unless the golden update environment
  variable is explicitly set.

The test suite should follow LB's pattern of using `CydMemory` both to exercise
the real shared application and to create reviewed previews. Tests must not
reimplement the expected UI in a separate host-only renderer.

## Startup and platform controls

DNS Tester must have a shared splash state that is rendered immediately after
display and calibration initialization and remains visible while Wi-Fi is being
initialized or connected. It may transition to a more specific shared Wi-Fi
setup screen when the captive portal becomes available. The normal DNS
dashboard must not appear until networking is ready; showing an apparently
usable tester while Wi-Fi startup is blocking is confusing and is considered a
bug. The splash renderer and its memory golden belong to examples core, modeled
on the LB clock splash pattern.

The DNS Tester WASM page must expose a simulated physical BOOT button outside
the CYD screen, modeled on the appropriate LB browser example. JavaScript sends
that press through `ButtonWasmSource`; it must not directly mutate calibration
or application state. The shared application handles it through the same
button/restart path used by hardware. Browser tests must prove that BOOT clears
calibration and starts calibration in landscape before restoring the saved
application orientation.

## Generated ESP examples

ESP board examples remain generated. The source of truth for DNS Tester and
Conway launchers must be the Jinja templates in the ESP examples crate.

The generated launchers should visibly be thin: construction, platform
adapters, one shared entry-point call, and error propagation. Any substantial
application logic found in a generated launcher should be treated as a failed
shared-code extraction.

Regeneration must be part of local and CI verification so editing only a
generated board file cannot appear to work temporarily.

## Migration plan

Implementation should proceed as a sequence of working, testable checkpoints.
Complete a phase, run its gate, and continue immediately to the next phase when
the gate passes. A failure stops further migration until it is corrected; do
not stack later structural changes on a failing checkpoint. `CydMemory` is the
primary executable specification until platform wrappers are migrated.

Golden images created at a gate must be inspected, not merely accepted because
the test update mechanism produced them. Intentional golden changes should be
reviewed together with the code that caused them.

### Phase 0: Capture the baseline

Work:

1. Run the existing DNS Tester and Conway host/memory tests.
2. Record the currently supported ESP, RP, and WASM build commands.
3. Preserve existing golden images for comparison during extraction.

Gate:

- Existing host tests pass before files are moved.
- Existing DNS Tester and Conway WASM crates build.
- Any pre-existing failure is documented separately and is not attributed to
  the reorganization.

### Phase 1: Establish examples core without changing behavior

Work:

1. Create `device-envoy-examples-core` as a workspace member.
2. Keep it `no_std` with no allocator.
3. Move the existing DNS Tester renderer and its current `CydMemory` preview
   test out of `device-envoy-core` without redesigning behavior.
4. Keep temporary platform callers compiling against the new canonical
   location only for the duration of the migration; do not add compatibility
   aliases in `device-envoy-core`.

Gate:

```text
cargo test -p device-envoy-examples-core
cargo check -p device-envoy-examples-core --no-default-features
```

- The moved CydMemory test renders the same baseline pixels.
- `device-envoy-core` no longer owns DNS Tester application rendering.
- All current DNS Tester consumers compile against examples core.

### Phase 2: Automate SVG-to-TGA assets

Work:

1. Move the authoritative SVG bundle under examples core.
2. Add a Rust `build.rs` and Rust build dependencies that produce the two TGA
   backgrounds without external executable dependencies.
3. Remove sample dynamic values and developer guides during generation.
4. Include the generated TGA bytes in the shared renderer and delete obsolete
   hand-generated DNS Tester backgrounds.
5. Add focused build-helper tests for dimensions, omitted layers, TGA format,
   and deterministic output where practical.

Gate:

- Delete the generated outputs and build examples core from a clean state.
- Build twice and confirm that the generated bytes are identical.
- CydMemory renders both generated backgrounds and matches reviewed 320x240 and
  240x320 golden PNGs.
- A fixture with invalid dimensions or missing required SVG metadata fails with
  a useful diagnostic.

### Phase 3: Make DNS Tester a complete shared application

Work:

1. Define shared state, inputs, results, and actions.
2. Move control rectangles and orientation hit testing into shared code.
3. Move counter/result transitions and DNS outcome handling into shared code.
4. Move splash, connecting, setup, ready, and unavailable screen decisions into
   shared code.
5. Move CAL, WI-FI, ROTATE, BOOT, calibration, and restart decisions into the
   same shared state machine.

Gate:

- A CydMemory scenario starts at the splash, advances through Wi-Fi connecting
  and ready, performs successful and failed DNS tests, and checks counters and
  reviewed frames at each meaningful state.
- Scripted ROTATE touches produce landscape, portrait, landscape inverted, and
  portrait inverted in that order, with a distinct reviewed golden for each.
- The control hitboxes work in all four orientations.
- Scripted CAL and BOOT enter landscape calibration and restore the saved
  application orientation after calibration.
- Scripted WI-FI emits the expected reset/setup action without platform code.
- Tests call the public shared application API used by platform runners.

This is the main behavior gate. Platform migration must not begin until it
passes.

### Phase 4: Migrate DNS Tester WASM

Work:

1. Reduce the WASM crate to browser plumbing and deterministic DNS completion.
2. Route canvas touches and `ButtonWasmSource` events into the shared app.
3. Keep the simulated BOOT button outside the CYD canvas.
4. Synchronize browser case dimensions and inversion styling from shared
   orientation state.

Gate:

- The WASM crate builds from a clean package directory.
- Browser tests exercise test taps, CAL, WI-FI, ROTATE, and BOOT.
- All four rotations match the CydMemory state sequence, including the two
  upside-down presentations.
- Calibration always appears in landscape and then restores the prior
  orientation.
- JavaScript contains no application counter, hitbox, or calibration policy.

### Phase 5: Migrate DNS Tester hardware runners

Work:

1. Change the ESP template to construct adapters and invoke the shared app.
2. Regenerate every supported ESP board example.
3. Change the RP example to invoke the same shared app.
4. Keep Wi-Fi and real DNS execution in platform adapters while feeding their
   events and results back to shared state.

Gate:

- Generated ESP and RP examples compile through normal local CI.
- Regenerating ESP examples leaves no uncommitted differences.
- Hardware starts on the splash rather than the ready dashboard while Wi-Fi is
  running.
- One ESP smoke test covers captive-portal setup, connection, and a real DNS
  result; perform the equivalent RP smoke test where supported.
- ROTATE, CAL, and WI-FI behavior agrees with the CydMemory scenarios.

### Phase 6: Migrate Conway through the same boundary

Work:

1. Move the implementation from `device-envoy-conway-core` into the Conway
   module of examples core.
2. Move Conway CydMemory tests and previews with the implementation.
3. Reduce Conway WASM, ESP, and RP launchers to platform plumbing.

Gate:

- Conway's deterministic CydMemory simulation and reviewed preview pass from
  examples core.
- Conway WASM and all supported hardware examples build.
- There is one Conway simulation and rendering implementation.

### Phase 7: Move platform examples into dedicated crates

Work:

1. Move ESP example manifests, templates, generated launchers, and relevant
   xtask support into `device-envoy-examples-esp`.
2. Move RP example launchers into `device-envoy-examples-rp`.
3. Keep `device-envoy-esp` and `device-envoy-rp` focused on reusable platform
   device implementations.
4. Update cargo aliases, `just` commands, documentation links, and CI paths.

Gate:

- Normal `just` commands build and run the examples from their new locations.
- Generator and CI path checks pass.
- Platform crates no longer contain complete application implementations.

### Phase 8: Remove obsolete paths and run full CI

Work:

1. Delete `device-envoy-conway-core` after all dependents use examples core.
2. Delete duplicated state, rendering, and control code from every wrapper.
3. Do not retain compatibility aliases or forwarding modules.
4. Delete or update superseded DNS Tester and Conway specs after release.

Gate:

```text
cargo xtask generate-board-examples
just check-all
```

- Regeneration leaves the worktree unchanged.
- No obsolete crate or application module remains referenced.
- Every acceptance criterion below is satisfied.

## Verification

At minimum, implementation must run:

```text
just check-all
cargo xtask generate-board-examples
```

The implementation should also directly verify:

- examples-core host tests and golden previews;
- a clean examples-core build after deleting its generated `OUT_DIR` assets;
- examples-core `no_std` build;
- DNS Tester and Conway WASM builds;
- WASM browser tests for calibration and all control actions;
- all supported ESP/RP example builds through normal local CI;
- one ESP hardware smoke test for DNS Tester Wi-Fi setup and DNS lookup;
- one RP hardware smoke test for each migrated application where supported.

No required target may be silently skipped because a toolchain component is
missing.

## Acceptance criteria

- DNS Tester state transitions and rendering have exactly one implementation.
- Conway simulation and rendering have exactly one implementation.
- ESP, RP, WASM, and `CydMemory` consume the same application modules.
- Platform wrappers contain no copied control coordinates or counter logic.
- Real and simulated DNS differ only behind the platform operation boundary.
- Calibration and orientation behavior are specified and tested once in shared
  application code.
- CydMemory drives ROTATE through landscape, portrait, landscape inverted, and
  portrait inverted and produces a sensible golden preview for each state.
- The shared Wi-Fi splash appears before the dashboard on hardware and in
  memory tests.
- The WASM BOOT control uses the simulated button source and shared application
  behavior rather than JavaScript-only state changes.
- DNS Tester design assets live together under examples core.
- A Rust `build.rs` deterministically generates the static TGA backgrounds from
  those SVG assets during a clean build.
- Memory golden tests render through the public shared application API.
- ESP generated examples survive regeneration without behavioral changes.
- `device-envoy-core` contains reusable abstractions rather than complete demo
  applications.
- The one-off `device-envoy-conway-core` crate is removed.
- `just check-all` builds every required target and passes.

## Non-goals

- Making browser Wi-Fi or DNS behave like unrestricted MCU networking.
- Combining DNS Tester and Conway into one deployed WASM binary.
- Moving simple API snippets into examples core when they do not have reusable
  application state.
- Generalizing every platform difference behind one large backend trait.
- Preserving obsolete crate names through compatibility shims.
