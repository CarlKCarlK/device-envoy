<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# DNS Tester Shared Runtime

## Purpose

Restructure the DNS Tester around the execution boundary used by the Linkage
Blaze examples. The examples-core crate should own the application startup,
rendering, and long-running execution loop. ESP, RP, WASM, and `CydMemory`
should provide platform resources and invoke that same core path.

This spec refines the runtime portion of
[`EXAMPLES_CRATE_REORGANIZATION_SPEC.md`](EXAMPLES_CRATE_REORGANIZATION_SPEC.md).
It corrects an architectural weakness in the first migration: the shared
the shared runtime must own state, input policy, rendering, and the service
loop rather than exposing an application object for each platform to drive.

## Linkage Blaze model

The Linkage Blaze examples establish this boundary:

```text
platform launcher
  -> construct Cyd/device resources and platform services
  -> call a shared splash/setup function
  -> call one shared application loop
```

Representative core entry points are:

- `clock_splash` followed by `clock`;
- `skeleton_clock_splash` followed by `skeleton_clock`;
- `ballet` for a display-only loop;
- `armatron` for a generic Cyd/touch/button loop.

The DNS Tester should follow the same shape. Its additional DNS and Wi-Fi
operations do not justify separate application loops; they are platform
services consumed by one shared loop.

## Current problem

The old architecture stopped at an event-driven state object:

```text
platform main -> initialize resources -> repeatedly drive a shared state object
```

ESP, RP, WASM, and memory tests therefore each own a version of the startup,
event dispatch, action handling, DNS completion, and redraw schedule. The
shared state transitions are useful, but they are not yet the application
entry point in the LB sense.

## Target architecture

The examples-core crate should expose one shared runtime boundary:

```text
ESP/RP/WASM/CydMemory adapter
  -> constructs display, touch, button, storage, Wi-Fi, and DNS sources
  -> calls device_envoy_examples_core::dns_tester::dns_tester(...)
```

The exact names may change, but the architecture must have:

- one shared async DNS Tester execution function;
- one shared splash/setup entry point, analogous to LB's clock splash
  functions;
- one shared rendering path;
- one shared event and action policy;
- platform adapters for real, browser, and scripted services.

The runtime keeps its changing state in local variables in the shared game
loop. Callers do not reproduce an event loop or drive an application object.

## Shared runtime responsibilities

The core runtime owns:

- showing the splash immediately after display initialization;
- showing Wi-Fi connecting/setup/unavailable notices;
- deciding when the dashboard becomes usable;
- reading calibrated touch and BOOT events from the supplied sources;
- applying touch hit-testing for all four orientations;
- applying CAL, WI-FI, ROTATE, ordinary test-tap, and BOOT policy;
- requesting calibration, Wi-Fi reset, orientation persistence, and reboot;
- requesting a DNS lookup and accepting its result;
- updating query, success, failure, and latency state;
- rendering static backgrounds and dynamic values;
- flushing the display at the same points for every platform.

The runtime must not know whether DNS is performed by `embassy-net`, simulated
by JavaScript, or scripted by `CydMemory`.

## Platform adapter responsibilities

Platform code remains responsible for constructing and adapting resources.
It should delegate existing device behavior rather than reimplement it.

### ESP and RP

ESP and RP launchers should:

- initialize board pins and clocks;
- construct the calibrated Cyd display/touch path using the existing `Cyd*`
  and calibration helpers;
- construct `Button*`, `FlashBlock*`, and `WifiAuto*` resources;
- provide one real DNS lookup operation and its measured duration;
- provide the platform reboot operation;
- invoke the shared splash/runtime entry points;
- translate only unavoidable platform error types.

They must not contain duplicated screen decisions, hitboxes, counters,
latency updates, or action-policy matches.

### WASM

The WASM crate should use the same runtime execution model as hardware:

- `CydWasm`, `ButtonWasmSource`, and `FlashBlockWasm` are platform adapters;
- browser animation frames provide scheduling;
- browser touch and BOOT events feed the same input source;
- the DNS operation returns a deterministic simulated result;
- JavaScript updates presentation details only and does not implement
  application policy.

The WASM wrapper may expose browser-friendly methods, but those methods should
start/feed/drive the shared runtime rather than become a second DNS Tester
state machine.

### CydMemory

`CydMemory` should execute the same shared runtime with scripted substitutes
for touch, BOOT, flash, Wi-Fi, DNS, reboot, and scheduling. Tests may stop the
scripted runtime with a finite test condition, but they must not replace the
runtime with a separate host-only renderer or manually reproduce its state
machine.

## Runtime service boundary

Introduce the smallest service abstraction needed to express operations that
are not already covered by Device Envoy traits. Do not create a large backend
trait that duplicates `Cyd`, `CydDisplay`, `Button`, `FlashBlock`, or
`WifiAuto`.

The service boundary must be able to represent:

- platform input/event availability;
- Wi-Fi startup and setup events;
- one DNS lookup with measured latency and success/failure;
- persistence and reboot actions;
- a finite scripted runtime for tests.

Prefer existing async traits and direct generic parameters where they fit.
Use a small adapter trait only for the remaining network, scheduling, and
restart differences.

## Migration phases

### Phase 1: Establish the shared entry points

Add a core splash function and a core DNS Tester runtime function. Keep the
runtime state local to that function. Write a short API-level test proving that the runtime can start at splash,
advance through Wi-Fi setup, and reach the dashboard.

### Phase 2: Build a scripted runtime

Implement a `CydMemory`-compatible scripted service adapter. Drive:

- splash;
- Wi-Fi connecting/setup/ready;
- successful and failed DNS lookups;
- all four ROTATE presentations;
- CAL, WI-FI, ordinary test taps, and BOOT;
- persisted orientation and calibration restart behavior.

Move the existing CydMemory golden scenarios onto this runtime.

### Phase 3: Migrate ESP and RP

Replace the duplicated ESP template and RP DNS event loops with calls to the
shared runtime. Keep only board construction, `WifiAuto`, real DNS, flash,
button, and reboot adaptation in those launchers. Regenerate and compile all
supported ESP examples and compile the RP examples.

### Phase 4: Migrate WASM

Make the WASM wrapper schedule and feed the same runtime. Remove browser-side
counter, screen, hitbox, calibration, and action policy. Keep only DOM/canvas,
browser storage, simulated DNS, and browser event adaptation.

### Phase 5: Remove the old boundary

Delete or make private any public API whose only purpose is to let platform
callers manually reproduce the runtime loop. Retain small pure state/rendering
helpers only where they are used internally by the shared runtime and tests.

## Acceptance criteria

- ESP, RP, WASM, and `CydMemory` invoke the same shared DNS Tester runtime.
- The core runtime owns splash, Wi-Fi notices, dashboard transition, input
  policy, DNS result updates, and rendering.
- Platform launchers contain no copied hitboxes, counters, latency policy, or
  screen-state decisions.
- Calibration and persisted orientation use existing Cyd/flash helpers, with
  only platform construction and reboot behavior outside the core runtime.
- Wi-Fi setup/reset uses existing `WifiAuto` behavior, with only service
  construction and platform error translation outside the core runtime.
- DNS success/failure and latency are supplied through one service boundary.
- The four orientation goldens and all state goldens run through the shared
  runtime, not direct test-only calls to a separate renderer.
- The WASM wrapper does not contain an independent DNS Tester state machine.
- ESP regeneration and all supported ESP/RP compile checks pass.
- `cargo check-all` verifies the shared runtime and generated examples.
- The implementation remains `no_std` and allocation-free in examples-core.

## Complexity characterization

The DNS-specific behavior is small:

- orientation selection is a finite sequence;
- calibration and persistence delegate to Cyd and flash helpers;
- Wi-Fi reset delegates to `WifiAuto`;
- DNS timing is one measured operation;
- counters and latency are a few state fields;
- rendering follows the existing static-image/dynamic-text pattern.

The substantial complexity in the current implementation is accidental:
the same orchestration is repeated across platform adapters. The purpose of
this spec is to move that orchestration into one LB-style core execution path,
not to add a more elaborate application framework.
