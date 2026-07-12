<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# DNS Tester WASM Demo

## Status

Planning only. This document describes the browser companion to the hardware
DNS tester; it does not authorize implementation by itself.

## Purpose

Provide a browser-hosted version of the DNS tester so the CYD display, touch,
calibration, orientation, reset controls, and status UI can be exercised
without hardware. The browser version should share as much application logic
as practical with the ESP and RP DNS testers.

The WASM demo is primarily a CYD/UI and interaction test. Browser Wi-Fi is not
part of the demo's scope.

## Visual parity with Linkage Blaze

The browser presentation should use the same visual language as the Linkage
Blaze CYD demos: the same page background, CYD case/background treatment,
layout proportions, typography, colors, control styling, and off-screen BOOT
control treatment. This is a companion demo, not a second web visual design.

Copy the required static resources from the Linkage Blaze web demo into the
Device Envoy version directory when the demo is created. The deployed Device
Envoy page must own a copy of its resources so historical versions remain
reproducible and do not depend on another repository's current branch.

The resource copy should include only the assets needed by the DNS tester. It
should preserve the Linkage Blaze attribution and licensing requirements, and
the source asset paths and originating Linkage Blaze version should be recorded
in the DNS tester web README or build notes.

## Relationship to the hardware DNS tester

The hardware versions test:

- Wi-Fi and touch operating together;
- Wi-Fi reset and captive-portal recovery;
- touch calibration and calibration reset;
- persistent orientation changes;
- display redraw and touch-coordinate behavior.

The WASM version preserves the display, touch, calibration, orientation, and
reset portions. It substitutes browser behavior for hardware-only services:

- `CydWasm` provides the CYD display and touch surface.
- `FlashWasm` provides persistent browser-side flash-block behavior.
- DNS requests use a browser-safe substitute or a deterministic demo result;
  they must not require the browser to access arbitrary DNS servers directly.
- The on-screen Wi-Fi reset action is disabled or visibly marked unavailable.

## Code organization

Create a dedicated WASM crate:

```text
crates/device-envoy-dns-tester-wasm/
```

The crate should expose a small `wasm-bindgen` API for the web page and use
`CydWasm` as the device implementation. Shared, platform-neutral DNS tester
state and UI behavior should move into a reusable core module when doing so
does not pull browser or platform dependencies into the `no_std` core crate.

The web shell, JavaScript bootstrap, CSS, images, and generated WASM package
should live under the versioned Pages tree:

```text
docs/dns-tester/
  index.html
  v1/
    index.html
    main.js
    pkg/
```

The generated `pkg/` contents are deployment artifacts produced by the WASM
build process. The source of truth remains the Rust WASM crate and the static
web files.

## CYD screen behavior

The browser CYD surface should match the hardware DNS tester:

- Tap Screen starts a DNS-test operation.
- The status area spells out query, success, failure, and latency values.
- The bottom controls are ordered left-to-right as `ROT`, `CAL`, `WiFi`.
- `Tap Settings:` appears immediately above the bottom controls.
- The Wi-Fi control remains present for UI parity but performs no operation in
  the browser; it may be disabled or show an unavailable message.
- Control hit-testing must use the same oriented screen coordinates as the
  rendered controls.

## Orientation

The ROT control cycles through all four orientations and persists the choice
through `FlashWasm`:

```text
Landscape → Portrait → LandscapeInverted → PortraitInverted → Landscape
```

The browser display should apply the selected orientation after rebooting or
reinitializing the WASM device, matching the hardware behavior. The shared
touch calibration itself is not orientation-aware; calibration is always
presented in landscape. The DNS tester transforms calibrated landscape touch
coordinates into the selected application orientation before hit-testing or
using app-level points.

## Calibration and reset controls

The browser version should support the same recovery paths as hardware where
the browser can provide an equivalent input:

- `CAL` clears the calibration block and restarts the app in landscape
  calibration mode.
- A BOOT control exists outside the visible CYD screen, matching the off-screen
  physical-button controls used by selected Linkage Blaze demos.
- BOOT provides the physical-button backup path for recalibration while the
  DNS tester is running.
- Calibration requires a touch/release cycle for each target, including the
  center confirmation target.

The off-screen BOOT control must not be confused with a CYD screen coordinate;
it is a browser-page control wired to the same generic button abstraction used
by the shared app flow.

## Browser persistence

`FlashWasm` should provide independent records for:

1. Wi-Fi state, if the shared record layout requires it, even though the WASM
   UI does not operate Wi-Fi;
2. touch calibration;
3. display orientation.

The browser implementation may expose a clear-storage or reload control for
development, but normal CAL and ROT behavior should exercise the same logical
flash-block operations as hardware.

## GitHub Pages and versioning

Device Envoy already publishes the Conway demo from `docs/` using versioned
directories such as `docs/conway/v1/` and `docs/conway/v2/`. Follow that
pattern for this demo:

- `docs/dns-tester/index.html` redirects to the current version.
- Each released web version lives permanently under `docs/dns-tester/vN/`.
- A new version is made by copying the previous version, rebuilding the WASM
  package, and updating the root redirect.
- Old version URLs remain valid after newer versions are published.

The intended public URLs are:

```text
https://carlkcarlk.github.io/device-envoy/dns-tester/
https://carlkcarlk.github.io/device-envoy/dns-tester/v1/
```

The existing repository documentation describes publishing `docs/` through
GitHub Pages. Add an automated deployment workflow only if the repository's
Pages configuration is changed from its current setup.

## Build and local preview

The implementation should provide a concise repeatable build command that:

1. builds the WASM crate for `wasm32-unknown-unknown`;
2. runs `wasm-bindgen` or `wasm-pack` with the web target;
3. places generated artifacts in the selected `docs/dns-tester/vN/pkg/`;
4. supports serving the `docs/` directory locally because ES-module WASM
   loading requires HTTP rather than a `file:` URL.

The build must be able to target a new version directory without overwriting
previous released web artifacts.

## Acceptance criteria

- The WASM DNS tester loads from a versioned Device Envoy GitHub Pages URL.
- The CYD display and touch interaction run through `CydWasm`.
- Calibration and orientation records use `FlashWasm` independently.
- Calibration always runs in landscape.
- ROT exercises all four orientations and persists across reinitialization.
- The bottom controls are ordered `ROT`, `CAL`, `WiFi`.
- The Wi-Fi control is visibly unavailable or inert without breaking the rest
  of the UI.
- The off-screen BOOT control resets calibration like the hardware backup
  button.
- Existing versioned URLs remain unchanged when a new version is published.
- The browser build and local preview have documented repeatable commands.
- The page background, CYD presentation, typography, controls, and BOOT
  treatment match the selected Linkage Blaze reference demo, with copied
  versioned resources checked into the Device Envoy Pages tree.
