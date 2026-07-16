<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# CYD WASM Browser Policy Fix

## Status

Implemented. This is a narrow corrective specification for the current CYD
WASM application-framework implementation. It does not replace the general
framework design in `CYD_WASM_APP_FRAMEWORK_SPEC.md`.

Where the current framework spec requires interactive browser touch
calibration, this corrective spec takes precedence. It restores the browser
policy already described in `CYD_WASM_CONSTRUCTION_MEDIUM_ARTICLE.md`: browser
pointer coordinates are intrinsically usable, so WASM has no calibration
construction phase.

Breaking changes to the unreleased WASM framework API are allowed. Do not keep
compatibility aliases or parallel command variants for the current interactive
calibration behavior.

## Objective

Starting a CYD application in the browser must never show, load, save, clear,
or otherwise participate in physical touch calibration.

The final browser story is:

1. A display-only core receives `CydDisplayWasm` through
   `start_cyd_display_web_app`.
2. A touch-capable core receives an immediately usable `CydWasm` through
   `start_cyd_web_app`.
3. Browser pointer coordinates use the simulator's intrinsic identity mapping.
4. If shared core code requests calibration, the browser explains that it is
   unnecessary and restarts the application normally.

The shared message must be:

> Calibration is not needed in the browser.

## Current incorrect behavior

The current implementation in `device-envoy-core::wasm::app` treats a browser
touch application like physical hardware:

- `start_cyd_web_app` opens a namespaced calibration flash block;
- the supervisor probes that block before running application code;
- missing calibration creates a temporary landscape simulator;
- the supervisor calls `ensure_calibration` and presents four physical-panel
  targets;
- `CydWebCommand::Recalibrate` clears calibration storage and repeats the flow;
- DNS Tester and Armatron browser tests now complete the physical calibration
  exercise.

This adds ceremony without improving browser input. `CydWasm::new` already
constructs `CydTouchWasm` with the identity calibration transform, which is the
correct browser adapter.

The capability-specific split itself is correct and must remain:

- DNS Tester and Armatron use `start_cyd_web_app` and receive `CydWasm`.
- Ballet, Clock, and Skeleton Clock use `start_cyd_display_web_app` and receive
  `CydDisplayWasm`.

## Required framework changes

### Touch-capable startup

`start_cyd_web_app` must:

1. Open only the application's orientation storage.
2. Load the saved orientation or use `initial_orientation`.
3. Construct `CydSimulatorWasm` directly in that application orientation.
4. Use the `CydWasm` returned by the simulator as already calibrated browser
   input.
5. Call `inner_main(&mut cyd, &mut button)` without an intervening calibration
   phase.

It must not:

- create or open `<namespace>/calibration`;
- import or call `ensure_calibration`;
- load or save `CalibrationConfig`;
- construct a temporary landscape session;
- expose physical calibration targets;
- reconstruct a CYD from artificially decalibrated parts.

Starting a touch-capable app with saved portrait orientation must construct the
first and only application session in portrait. There must be no transient
320x240 landscape session.

Existing stale `<namespace>/calibration` browser-storage entries may be ignored.
Do not add migration code merely to delete them, because the final runtime must
not access calibration storage at all.

### Display-only startup

Keep `start_cyd_display_web_app` and its typed `CydDisplayWasm` callback. It must
continue to have no calibration code or calibration storage.

Do not merge the two entry points behind a Boolean configuration field. Their
callback types document the generic core's required capability.

### Calibration request command

`CydWebCommand` is a browser-platform command, so its name must describe the
browser policy. Replace `Recalibrate` with:

```rust,no_run
pub enum CydWebCommand {
    Restart,
    CalibrationNotNeeded,
    ResetWifi,
    Reorientate(Orientation),
    Stop,
}
```

When the supervisor receives `CalibrationNotNeeded`, it must:

1. release transient pointer and BOOT state;
2. enqueue one informational typed notice with the stable ID
   `calibration-not-needed`;
3. reconstruct and restart the application in its current orientation.

It must not modify any browser storage.

The shared JavaScript shell's default text for
`calibration-not-needed` must be exactly:

```text
Calibration is not needed in the browser.
```

The notice should use the ordinary finite informational-notice duration. It is
not fatal and must not require application-specific JavaScript configuration.

Both touch and display supervisors should handle the command exhaustively. A
display-only application should not normally return it, but handling it as the
same informational restart is preferable to introducing a fatal error for a
harmless platform-policy request.

### Application mappings

The browser `inner_main` functions remain concise and translate core policy at
one adjacent exhaustive match.

DNS Tester:

```rust,no_run
match dns_tester::run(cyd, button, &mut dns).await? {
    CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded),
    CoreExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
    CoreExit::Reorientate(orientation) => {
        Ok(CydWebCommand::Reorientate(orientation))
    }
}
```

Armatron:

```rust,no_run
match armatron(cyd, button).await? {
    ArmatronExit::CalibrationRequested => {
        Ok(CydWebCommand::CalibrationNotNeeded)
    }
}
```

Do not change the shared generic core exit enums. Physical ESP and RP callers
still interpret those exits as real hardware calibration requests.

### Remove obsolete WASM-only calibration plumbing

After removing supervisor calibration, delete WASM-only methods that have no
remaining caller, including `CydWasm::from_parts` and
`CydWasm::parts_uncalibrated` if repository-wide search confirms they are
unused.

Do not remove the memory backend's independently used uncalibrated-parts API or
any physical ESP/RP calibration support.

## Simulated DNS latency

A browser cannot ask JavaScript for the raw DNS answer or OS resolver timing of
an arbitrary hostname. Browser networking deliberately exposes higher-level
HTTP operations instead. The demo can, however, simulate a DNS capability and
its latency accurately at the capability boundary.

`DnsFixedWasm::resolve` currently returns synchronously, so the shared DNS
Tester correctly measures `0 ms`. Change the WASM resolver to wait for a real,
deterministic 12 ms before returning its fixed addresses:

```rust,no_run
use embassy_time::{Duration, Timer};

const SIMULATED_DNS_LATENCY: Duration = Duration::from_millis(12);

async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
    Timer::after(SIMULATED_DNS_LATENCY).await;
    Ok(self.addresses.clone())
}
```

Do not special-case the rendered latency text and do not change the shared
`Dns` trait. The generic DNS Tester must continue to measure elapsed time around
the capability call. Browser scheduling can occasionally make the displayed
measurement slightly greater than 12 ms; the implementation must provide a
real 12 ms minimum delay rather than claim an exact duration it did not wait.

Keep the fixed loopback response. This remains an honest simulated resolver,
not a claim that the browser performed a native DNS query.

## Documentation corrections

Update `CYD_WASM_APP_FRAMEWORK_SPEC.md` so it no longer requires:

- calibration storage for touch-capable WASM applications;
- temporary landscape calibration sessions;
- interactive browser calibration tests;
- `CydWebCommand::Recalibrate`.

Document the two capability-specific entry points and the
`CalibrationNotNeeded` notice policy instead.

Keep `CYD_WASM_CONSTRUCTION_MEDIUM_ARTICLE.md` unchanged except for separately
approved editorial work. Its statement that WASM has no calibration
construction phase is the intended behavior.

## Required tests

### Rust/WASM tests

- Starting DNS Tester with empty local storage calls its application path
  without creating a calibration record.
- Starting DNS Tester with saved portrait orientation immediately produces a
  240x320 canvas and never switches to landscape.
- `start_cyd_web_app` can deliver browser touch input without a stored
  `CalibrationConfig`.
- Returning `CalibrationNotNeeded` queues one informational notice with ID
  `calibration-not-needed` and restarts with a stable handle.
- The display-only entry point still starts without touching calibration
  storage.
- `DnsFixedWasm::resolve` returns the configured fixed addresses only after at
  least 12 ms of Embassy time has elapsed.

Delete tests whose purpose is to tap four browser calibration targets or prove
that browser calibration temporarily uses landscape orientation.

### Browser tests

- Fresh-context DNS Tester opens directly to its dashboard.
- A center DNS action performs one lookup and updates query/success state; it
  never opens calibration.
- The displayed simulated latency is at least `12 ms`, allowing a reasonable
  upper bound for loaded CI machines rather than requiring exactly `12 ms`.
- Pressing `CAL` shows `Calibration is not needed in the browser.` and returns
  to the dashboard in the same orientation.
- Pressing BOOT when the shared core reports its calibration exit follows the
  same notice-and-restart policy.
- Armatron opens directly to the application and its controls work without a
  calibration prelude.
- Ballet, Clock, and Skeleton Clock continue to open directly with no
  calibration UI.
- No tested browser context creates a namespaced calibration-storage key.

Tests must wait on visible state or typed notices. Do not coordinate them with
large fixed animation-frame counts or sleeps intended to hide lifecycle races.

## Generated artifacts and validation

After implementation:

1. Regenerate the DNS Tester WASM package and browser files.
2. Rebuild all four Linkage Blaze WASM pages.
3. Run Device Envoy core and DNS Tester WASM checks.
4. Run all four Linkage Blaze WASM checks.
5. Run the repository browser-test commands with their required local server.
6. Run Device Envoy `cargo check-all` and Linkage Blaze `just check-all`.
7. Run `git diff --check` in both repositories.

The existing Ballet long-running const-evaluation warning is not part of this
fix and must not be suppressed.

## Acceptance criteria

- No WASM application displays physical touch-calibration targets.
- No WASM runtime path opens, reads, writes, or clears calibration storage.
- `CydWasm` remains a real calibrated-touch capability backed by intrinsic
  browser coordinates.
- Display-only applications continue to receive only `CydDisplayWasm`.
- DNS Tester and Armatron start immediately in their application orientation.
- CAL and calibration-related BOOT exits produce the exact shared informational
  message and restart normally.
- Simulated DNS waits 12 ms at the resolver boundary and the generic core
  reports the measured latency.
- ESP, RP, memory, and shared generic-core calibration behavior is unchanged.
