<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# CYD example button semantics

Status: accepted behavior contract and implementation checklist. The checkboxes below track the work required to make the five CYD examples coherent across WASM, ESP, and RP.

## Autonomous execution rule

When work is being performed autonomously, do not stop merely because the next
step is known or belongs to the agent. Continue through all safe, in-scope
implementation, test, documentation, and validation steps. Stop only when the
work is complete, a real external blocker requires user input, or continuing
would exceed the authorized scope. At every genuine stopping point, report the
status, remaining work, and why the next safe step was not performed.

This spec covers the four Linkage Blaze examples (Armatron, Ballet, Clock, and Skeleton Clock) and Device Envoy's DNS Tester. It treats the physical CYD BOOT button and the simulator's `#boot-button` as the same app input. Browser-only controls, such as the Clock time setter, are extensions and are listed separately; they must not silently redefine the physical button.

## Goals and rules

Every app/platform combination must document BOOT behavior in every state in which input can arrive:

- cold start and splash;
- touch calibration;
- Wi-Fi setup/connection, where applicable;
- the steady/main state;
- an active operation or animation;
- restart, failure, or transition back to another state.

The contract is:

- A short BOOT tap has one intentional meaning in each state. It must not be an undocumented no-op.
- A held button is consumed once per logical action. Re-entering a state must not replay the same press because of a stale level or queued simulator event.
- A BOOT action that changes calibration, orientation, or Wi-Fi must persist the intended setting before restarting or changing state.
- An on-screen button is an app control, not a second physical BOOT button. If it has the same effect as BOOT, the implementation should route both inputs through the same typed action.
- WASM should model the behavior that matters to the hardware app. Browser-only conveniences may add controls, but may not hide missing hardware semantics.
- No app should terminate its input loop for an informational notice. Fatal errors are the exception.

## Current inventory at the time this spec was written

| App | WASM now | ESP/RP now |
| --- | --- | --- |
| Armatron | BOOT is passed to the core loop and requests recalibration; `cal` does the same. Fresh WASM sessions seed default calibration. No Wi-Fi. | BOOT and `cal` request recalibration; calibration is cleared from flash and the app reboots into calibration. No Wi-Fi. |
| Ballet | The simulator button is discarded. There are no app touch buttons and no simulated Wi-Fi. | The launchers do not create an app BOOT input. There are no app touch buttons and no Wi-Fi. |
| Clock | The simulator button is discarded after startup. The splash is followed directly by the clock loop; there is no simulated Wi-Fi. The time setter is browser-only. | A physical button is supplied to `WifiAuto` to force the captive portal during startup. The clock loop does not receive a BOOT button. The time UI has no CYD touch buttons. |
| Skeleton Clock | Same as Clock: the button is discarded, startup goes directly through the splash into the clock loop, and there is no simulated Wi-Fi. The time setter is browser-only. | A physical button is supplied to `WifiAuto` during startup only. The steady-state skeleton-clock loop does not receive it. There are no CYD touch buttons. |
| DNS Tester | Calibration, simulated Wi-Fi, and the main loop are separate states. BOOT during simulated Wi-Fi requests Wi-Fi reset; BOOT in the main loop requests recalibration. Main-screen `CAL`, `WiFi`, and `ROT` controls map to calibration, Wi-Fi reset, and orientation change. | The same broad mapping exists: BOOT during Wi-Fi setup resets Wi-Fi; BOOT in the main loop is a calibration backup; `CAL`, `WiFi`, and `ROT` perform the corresponding actions. RP and generated ESP launchers persist changes and restart as needed. |

The Clock and Skeleton Clock WASM implementations therefore do **not** currently have simulated Wi-Fi. That is a required change, not an optional browser feature. ESP and RP already exercise real Wi-Fi setup for both clocks.

## Accepted design decisions

The following decisions are confirmed:

- Ballet BOOT restarts the animation from its initial deterministic frame.
- Clock and Skeleton Clock BOOT in the main state reset Wi-Fi and re-enter captive-portal setup.
- BOOT during calibration restarts calibration after the current press is consumed and released.
- DNS Tester BOOT during an active DNS lookup safely cancels or finishes the lookup, then enters recalibration.
- Clock and Skeleton Clock have no on-screen CYD controls; browser time setters remain simulator-only extensions.
- Ballet has no calibration state.

## Canonical state names

The matrices below use these state names. An implementation may use different Rust types, but its externally observable behavior must match them.

| State | Meaning |
| --- | --- |
| `Startup` | Process/device launch, before the app's main screen is ready. |
| `Calibration` | Touch calibration or a calibration restart is in progress. Apps without calibration still need an explicit startup policy. |
| `WifiSetup` | Captive portal, connection attempt, retry, or simulated connection. |
| `Main` | The app's stable, normal display. |
| `Active` | A user-started operation, animation, hold action, or other transient activity. |
| `Transition` | A requested restart, orientation change, recalibration, Wi-Fi reset, or error recovery is being applied. |

## App 1: Armatron

Armatron's on-screen controls are `prev`, `next`, reverse-kinematics play/stop, reverse-kinematics step-and-hold, and `cal`.

### Behavior matrix and checklist

| Platform/state | BOOT: current behavior | BOOT: required behavior | On-screen controls: current behavior and required behavior |
| --- | --- | --- | --- |
| WASM `Startup` | Seeds default calibration when storage is empty, then enters calibration. | Keep the deterministic startup policy; a BOOT press before the loop is ready must be consumed, not replayed into the next state. | None before the app screen. |
| ESP/RP `Startup` | Enters calibration when calibration is unavailable; otherwise starts Armatron. | Match WASM's documented startup policy and consume a press spanning startup. | None before the app screen. |
| All platforms `Calibration` | The calibration flow owns the button; a valid calibration is saved. | A BOOT tap may restart/cancel the current calibration only if the calibration UI says so; otherwise ignore it while showing progress. Do not leave a stale press for Armatron. | No Armatron controls should be active. |
| All platforms `Main` | BOOT returns `CalibrationRequested`; the WASM path clears storage and loops, while hardware clears flash and reboots. | Keep BOOT as “recalibrate,” with the same user-visible transition and persistence semantics on all platforms. | `cal` requests the same recalibration action. `prev`/`next` select the previous/next target. Play/stop starts or stops reverse-kinematics playback. Step advances while held. The shared mappings are implemented; cross-platform acceptance tests remain. |
| All platforms `Active` | BOOT is checked by the Armatron loop and requests recalibration; touch controls continue to drive playback/step behavior. | BOOT must stop the active operation, release any held-step state, and enter calibration exactly once. | A new target selection must not corrupt an active step/playback operation. `cal` has the same priority as BOOT. |
| All platforms `Transition` | WASM releases the triggering input before calibration; hardware restarts after clearing flash. | Verify release/debounce behavior on ESP, RP, and WASM and add an automated regression for a held BOOT tap. | Verify `cal` follows the same transition path as BOOT. |

Checklist:

- [x] Document the existing BOOT and `cal` recalibration path.
- [x] Add a shared semantic action/test for BOOT and `cal`.
- [x] Define and test BOOT during calibration and playback, including held input.
- [x] Add deterministic core tests for play/stop, pressed step, and clearing an active run.
- [x] Add deterministic core coverage for `prev`/`next` seed wraparound.
- [x] Add WASM browser coverage for `prev`, `next`, play/stop, and step input wiring.
- [ ] Verify `prev`, `next`, play/stop, and step behavior on WASM, ESP, and RP.
- [x] Verify the simulator's full-screen and normal-mode controls expose the same BOOT input.

## App 2: Ballet

Ballet is currently a continuous, non-interactive animation with no on-screen CYD controls.

### Behavior matrix and checklist

| Platform/state | BOOT: current behavior | BOOT: required behavior | On-screen controls |
| --- | --- | --- | --- |
| WASM `Startup`/`Main`/`Active` | The simulator creates a button but the Ballet WASM entrypoint discards it. BOOT is a no-op. | A short BOOT tap must restart the ballet at its initial deterministic pose/frame. It must be safe during animation and must not create multiple animation tasks. | None currently. Keep the screen free of controls unless a future Ballet action is designed. |
| ESP/RP `Startup`/`Main`/`Active` | The launchers do not create or pass an app BOOT button. | Add the board's CYD BOOT input and make it restart the same deterministic initial pose/frame as WASM. | None. |
| All platforms `Calibration`/`Transition` | No app-specific state exists today. | If startup calibration is ever added, BOOT must restart calibration; otherwise document that Ballet has no calibration state. A restart must release/debounce the input before animation resumes. | None. |

Checklist:

- [x] Decide whether “restart animation” is the released Ballet BOOT action and record the decision in the core API.
- [x] Add BOOT handling to WASM, ESP, and RP.
- [x] Add a deterministic restart test and a browser test.
- [x] Confirm Ballet has no application on-screen controls or Wi-Fi requirement; only shared simulator UX remains.

## App 3: Clock

Clock has no CYD touch buttons. The browser time setter changes the simulated time and is not a hardware control.

### Behavior matrix and checklist

| Platform/state | BOOT: current behavior | BOOT: required behavior | On-screen/browser controls |
| --- | --- | --- | --- |
| WASM `Startup`/`WifiSetup` | Shared simulated Wi-Fi drives the splash/setup flow; the simulator button can request a reset/re-entry. | Keep the shared setup flow and consume BOOT once per press. | The time setter remains a browser extension and changes the clock source only. It must not be presented as a CYD hardware button. |
| WASM `Main` | BOOT requests Wi-Fi reset/re-entry to setup through the typed clock exit. | Keep the controlled return to `WifiSetup`, with a notice and no duplicate loop. | No CYD touch buttons. |
| ESP/RP `Startup`/`WifiSetup` | BOOT is passed to `WifiAuto` and can force the captive portal. | Keep this behavior and make the state transition visible and testable. | No CYD touch buttons. |
| ESP/RP `Main` | The button is no longer passed to the clock loop, so BOOT has no app meaning after Wi-Fi startup. | Retain the same physical button and make a short BOOT tap reset Wi-Fi/re-enter captive-portal setup. | No CYD touch buttons. |
| All platforms `Active`/`Transition` | No separate clock action currently consumes BOOT. | BOOT has the same Wi-Fi-reset meaning while the clock is rendering; do not tear down a frame or spawn a second loop. | The time setter may update the source while the clock runs; it must not bypass the Wi-Fi-reset action. |

Checklist:

- [x] Add simulated Wi-Fi to Clock WASM using the shared simulator Wi-Fi path and notices.
- [x] Define the typed Clock action for “reset Wi-Fi” and use it from BOOT on all platforms.
- [x] Pass or otherwise route BOOT through the ESP/RP steady-state clock loop.
- [x] Add shared-loop tests for setup entry, connected rendering, and BOOT reset after a rendered tick; WASM browser coverage verifies re-entry.
- [x] Add browser coverage for the main-state BOOT reset and re-entry.
- [x] Add independent browser coverage for the time setter.

## App 4: Skeleton Clock

Skeleton Clock has the same input gap as Clock. Its browser time setter is an extension, not a CYD button.

### Behavior matrix and checklist

| Platform/state | BOOT: current behavior | BOOT: required behavior | On-screen/browser controls |
| --- | --- | --- | --- |
| WASM `Startup`/`WifiSetup` | Shared simulated Wi-Fi drives the splash/setup flow; the simulator button can request a reset/re-entry. | Keep the shared setup flow and consume BOOT once per press. | The time setter changes simulated time only. No CYD touch buttons. |
| WASM `Main` | BOOT requests Wi-Fi reset/re-entry to setup through the typed skeleton-clock exit. | Keep the same notices and lifecycle as Clock. | No CYD touch buttons. |
| ESP/RP `Startup`/`WifiSetup` | BOOT is used by `WifiAuto` to force the captive portal. | Keep and test this behavior. | No CYD touch buttons. |
| ESP/RP `Main` | The button is not passed to the steady-state skeleton-clock loop, so BOOT is a no-op after startup. | Make a short BOOT tap reset Wi-Fi/re-enter captive-portal setup. | No CYD touch buttons. |
| All platforms `Active`/`Transition` | No distinct BOOT action exists. | BOOT must request the Wi-Fi reset once and safely restart the skeleton-clock lifecycle. | The time setter must remain independent from BOOT. |

Checklist:

- [x] Add simulated Wi-Fi to Skeleton Clock WASM using the shared simulator Wi-Fi path and notices.
- [x] Define the typed Skeleton Clock “reset Wi-Fi” action.
- [x] Route BOOT through the ESP/RP steady-state loop.
- [x] Add shared-loop tests for setup entry, connected rendering, and BOOT reset after a rendered tick; WASM browser coverage verifies re-entry.
- [x] Add browser coverage for the main-state BOOT reset and re-entry.
- [x] Add independent browser coverage for time-setting behavior.

## App 5: DNS Tester

DNS Tester has three on-screen settings buttons: `CAL`, `WiFi`, and `ROT`. Tapping the main test area starts the DNS lookup. `ROT` advances through the four display orientations and persists the next orientation.

### Behavior matrix and checklist

| Platform/state | BOOT: current behavior | BOOT: required behavior | On-screen controls: current and required behavior |
| --- | --- | --- | --- |
| WASM `Startup`/`Calibration` | Startup calibration is real simulator calibration. A calibration result is persisted. The current entrypoint has explicit input-release handling for restart paths. | BOOT must restart the current calibration cleanly or be ignored with a visible calibration-state rule; it must not skip points or leak into Wi-Fi/main. | No main-screen controls should act before calibration completes. |
| ESP/RP `Startup`/`Calibration` | Calibration is persisted; RP/ESP wait for BOOT release before restarting after calibration. | Preserve this behavior and test a press held across the calibration-to-Wi-Fi boundary. | No main-screen controls before the main state. |
| WASM/ESP/RP `WifiSetup` | BOOT asks the Wi-Fi layer to reset/re-enter setup. On WASM this is simulated; on hardware it resets the captive portal. | Keep the mapping, announce the transition, and ensure the same tap cannot also trigger recalibration in the next state. | `WiFi` must perform the same reset action where the main UI is available. |
| WASM/ESP/RP `Main` | BOOT requests recalibration. `CAL` requests recalibration. `WiFi` resets Wi-Fi. `ROT` persists the next orientation and restarts/reorients. The main touch area starts a DNS lookup. | Keep these meanings and make them one shared typed action table across all platforms. | `CAL` = clear/re-run calibration; `WiFi` = reset/re-enter Wi-Fi setup; `ROT` = advance/persist orientation; main test area = run DNS lookup. |
| WASM/ESP/RP `Active` DNS lookup | The core loop continues to render the lookup result and checks BOOT at loop boundaries. | BOOT must cancel or safely finish the active lookup, then enter recalibration exactly once. `WiFi`, `ROT`, and `CAL` must not be lost if touched at a defined polling boundary. | Keep the main test action distinct from settings controls; a settings tap must not start a DNS lookup. |
| All platforms `Transition` | Recalibration, Wi-Fi reset, and orientation changes restart or return through platform-specific code. | Persist first, release/debounce input, show a notice where useful, and return to exactly one known state. | Ensure each on-screen control is disabled or ignored until the transition completes. |

Checklist:

- [x] Document and preserve the current BOOT/Wi-Fi/calibration mapping.
- [x] Keep WASM simulated Wi-Fi and typed notices aligned with hardware semantics.
- [x] Clear WASM calibration storage before a BOOT/`CAL` recalibration restart.
- [x] Re-run browser coverage for BOOT recalibration, including a second calibration.
- [x] Add shared host-platform action/state tests for BOOT, `CAL`, `WiFi`, and `ROT` across all orientations; hardware runtime acceptance remains separate.
- [x] Add explicit active-DNS-operation coverage for BOOT and settings taps.
- [x] Verify the ESP and RP generated examples against the same transition checklist.

## Cross-platform implementation status

Implementation log:

- Added the autonomous-work handoff rule to both repositories' `AGENTS.md` files.
- Made Ballet's shared loop consume BOOT and restart its motion sequence; wired WASM, generated ESP launchers, and RP to the physical/simulated button.
- Added typed `ResetWifi` exits to the shared Clock and Skeleton Clock loops; wired steady-state BOOT reset through ESP/RP reboot paths.
- Added deterministic simulated Wi-Fi setup, status rendering, BOOT interruption, and re-entry to both Clock and Skeleton Clock WASM launchers.

The core implementation phase for all five examples is complete. Remaining work is platform acceptance coverage and final checklist closure:

1. Complete Armatron cross-platform acceptance for target navigation and reverse-kinematics controls.
2. Complete DNS Tester generated ESP/RP runtime acceptance tests.
3. Verify the simulator's normal/full-screen control presentation and Ballet's absence of accidental controls.

### Native Clock test-harness rule

The Clock and Skeleton Clock WASM implementations use Embassy's WASM timer
driver and WASM executor, so their browser behavior is valid and must remain
the integration authority. Native `cargo test` uses Rust's
`futures_executor::block_on`, while the shared debounced `Button` implementation
uses Embassy timers. Mixing those runtimes causes either an unresolved
`__pender` symbol or an incompatible-waker panic.

The native tests must therefore:

- use test-only Button doubles that override `wait_for_press()` and do not
  create Embassy timer futures;
- run any test that genuinely exercises Embassy timers on an Embassy-compatible
  native executor; and
- keep rendering/action tests deterministic and timer-free.

Do not add a fake no-op `__pender` implementation. It masks the link problem
and produces a less useful runtime failure when the waker belongs to the wrong
executor. Browser tests remain responsible for real debounce, browser timing,
simulated Wi-Fi, and BOOT re-entry.

Validation completed during this implementation pass:

- Device Envoy `cargo check-all` passed.
- Linkage Blaze core/WASM package checks passed.
- Linkage Blaze core unit tests passed: 129 tests.
- Device Envoy examples-core tests passed: 17 tests, including DNS control hit-testing and layout invariants.
- Device Envoy DNS Tester WASM library tests passed: 1 test, covering control hitboxes across orientations.
- Linkage Blaze's feature-gated Ballet core test passed, comparing the post-BOOT frame with the deterministic baseline.
- Linkage Blaze's feature-gated Armatron calibration-control test passed.
- Linkage Blaze's feature-gated Armatron BOOT-to-calibration test passed.
- Armatron's Playwright BOOT test passed with a held 700 ms simulator press.
- Generated ESP examples were checked for Ballet button wiring and Clock/Skeleton Clock `ResetWifi` routing; RP Clock/Skeleton Clock launchers were checked for the same exit path.
- Linkage Blaze `just check-all` passed end-to-end, including 138 feature-complete core tests, all ESP example builds, all RP example checks, and WASM utility builds.
- The native Clock/Skeleton Clock test harness now uses timer-free Button doubles plus a dev-only Embassy host executor dependency; no fake `__pender` implementation was added.
- Linkage Blaze `just test-cyd-browser` passed all 16 browser tests across the shared shell, full-screen BOOT, Armatron controls, Ballet, Clock, Skeleton Clock, DNS Tester, Wi-Fi, calibration, BOOT, ROT, CAL, and time controls.
- Full-screen mode now moves the shared BOOT control with the canvas and restores it on exit; the browser regression passed.
- The Armatron WASM browser test passed for `prev`, `next`, reverse-kinematics play/stop, and step input; ESP/RP runtime control acceptance remains open.
- Linkage Blaze's Clock/Skeleton Clock feature suite passed 135 tests, including BOOT before the first tick and after a rendered steady-state tick.
- Targeted Playwright BOOT/re-entry tests passed for Ballet, Clock, and Skeleton Clock.
- DNS Tester Playwright coverage passed, including BOOT immediately after starting a lookup and the subsequent calibration/re-entry path.
- DNS Tester Playwright coverage also passed for the WiFi, ROT, and CAL dashboard controls, including CAL after portrait reorientation.
- The all-pages build produced the WASM packages and the host-side preview test passed after the timer-free native test doubles and dev-only Embassy host executor dependency were added; no fake `__pender` implementation was added.

## Verification commands

From Device Envoy:

```text
cargo check-all
```

From Linkage Blaze:

```text
just check-all
```

For browser behavior, run the repository's Playwright suite after building the relevant WASM artifacts. The Clock and Skeleton Clock suites must include at least: startup, simulated Wi-Fi setup, BOOT reset from setup, BOOT reset from the main state, and independent time-setter behavior.

Suggested commit message:

```text
specify CYD example boot and touch semantics
```
