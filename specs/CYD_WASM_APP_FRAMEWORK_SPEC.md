<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# CYD WASM Application Framework

## Status

Planning only. This specification defines the preferred final API and migration
for the four Linkage Blaze CYD examples and the Device Envoy DNS Tester. Breaking
changes to the existing WASM crates and JavaScript protocol are allowed. Do not
keep compatibility shims for the current application-specific lifecycle APIs.

The existing `CYD_WASM_SIMULATOR_SPEC.md` remains the source for the physical
browser presentation and low-level simulator behavior. This specification adds
the missing application framework above `CydSimulatorWasm`. The existing
`CYD_WASM_CONSTRUCTION_MEDIUM_ARTICLE.md` must remain unchanged until this work
is implemented; revise or replace that article only after the final API exists.

## Objective

Make bringing a generic CYD application to a web page as direct as bringing it
to ESP or RP.

The teaching story must be:

1. Write an async core function against the capabilities it needs.
2. Write a WASM `inner_main` that constructs any browser-specific capabilities
   and calls the same core function.
3. Translate the core function's returned `Exit` into a small framework command.
4. Export one `start` function that gives the framework the canvas ID,
   presentation configuration, and `inner_main`.
5. Mount the returned standard handle with the shared JavaScript CYD shell.

For example, the complete application shape should be recognizable as:

```rust,no_run
const WEB_APP: CydWebAppConfig = CydWebAppConfig::new(
    "linkage-blaze/armatron",
    Orientation::Landscape,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, inner_main)
}

async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, ArmatronError<Infallible>> {
    match armatron(cyd, button).await? {
        ArmatronExit::CalibrationRequested => Ok(CydWebCommand::Recalibrate),
    }
}
```

This is the target level of ceremony. The exact generic bounds of
`start_cyd_web_app` may change as required by Rust's `AsyncFnMut` syntax, but
the application-facing shape must not become a builder, macro language, boxed
application trait, or app-specific wrapper class.

## Why the current launchers are not the desired API

The shared core boundary is already good. The current WASM launchers are not.
They repeatedly perform work that belongs to a browser platform runtime:

- query `window` and `document` and cast an element to `HtmlCanvasElement`;
- construct and split `CydSimulatorWasm`;
- call `spawn_local`;
- retain simulator controls across asynchronous work;
- wait for BOOT release before restarting loops;
- report fatal errors directly to `console`;
- resize canvases and duplicate orientation state;
- expose application-specific polling and string results to JavaScript;
- coordinate simulated reset and persistent browser storage;
- duplicate browser clock and Wi-Fi simulation code.

DNS Tester makes the problem most visible through its long-lived
`DnsTesterWeb`, `Rc<Cell<_>>`, `RefCell<_>`, `take_exit`, `reboot`, and manual
state synchronization. The Linkage Blaze launchers are smaller, but each still
owns a partial lifecycle loop and has subtly different error, reset, and Wi-Fi
behavior.

These differences are not application behavior. They are one unextracted WASM
platform lifecycle.

## Design principles

### Preserve the capability boundary

The framework must not replace `Cyd`, `CydDisplay`, `Button`, `ClockSync`,
`Dns`, or other focused capabilities with a universal web application trait.
The platform-neutral functions remain independently generic:

- Armatron requires `Cyd + Button`;
- Ballet requires `CydDisplay + Button`;
- Clock and Skeleton Clock require `CydDisplay + ClockSync + Button`;
- DNS Tester requires `Cyd + Button + Dns`.

`inner_main` is construction and platform policy, not a second application
abstraction.

### Put lifecycle mechanics in the framework

The framework owns canvas lookup, simulator construction, task spawning,
stable browser input control, restarts, transient input release, framework
storage, orientation persistence, calibration restart, simulated Wi-Fi reset,
fatal reporting, and host-request coordination.

An application crate must not need `Rc`, `Cell`, `RefCell`, `spawn_local`, a
manually polled exit slot, or a JavaScript string protocol.

### Keep construction explicit

Use one direct configuration constructor and one start function. Do not use a
builder. Do not hide application-specific resources inside the framework.

If an application needs `ClockSyncWasm`, deterministic browser DNS, or another
browser capability, construct it visibly in `inner_main` and pass it to the
generic core.

### Let Rust own application policy

JavaScript presents the virtual board and forwards browser input. Rust runs the
application and handles its typed lifecycle commands. JavaScript must not
interpret an application's `Exit` enum or decide how to restart it.

### Keep a stable JavaScript handle

Reorientation, calibration, and reset may replace the underlying `CydWasm` and
input sources. The handle returned to JavaScript must remain stable across all
such replacements. The shared shell binds input once.

## Architecture

```text
generic core function
  armatron / ballet / clock / skeleton_clock / dns_tester
                         |
                         v
                app-specific inner_main
       construct ClockSyncWasm, DnsFixedWasm, etc.
       call core and map Exit -> CydWebCommand
                         |
                         v
                 CYD web supervisor
   canvas + CydSimulatorWasm + persistence + restart loop
                         |
                         v
                stable CydWebAppHandle
       touch / BOOT / notices / host lifecycle requests
                         |
                         v
              shared JavaScript CYD shell
```

Add the framework in `device-envoy-core` behind the existing `wasm` feature.
Use `src/wasm/app.rs` or an equally direct module name; do not create a
`mod.rs` file.

`CydSimulatorWasm` remains the lower-level device constructor and escape hatch.
The new application framework becomes the documented default for complete CYD
web applications.

## Rust API

### `CydWebAppConfig`

Provide a small copyable configuration value with a direct constructor:

```rust,no_run
pub struct CydWebAppConfig {
    pub storage_namespace: &'static str,
    pub initial_orientation: Orientation,
    pub background: Rgb888,
    pub foreground: Rgb888,
    pub font: &'static MonoFont<'static>,
}

impl CydWebAppConfig {
    pub const fn new(
        storage_namespace: &'static str,
        initial_orientation: Orientation,
        background: Rgb888,
        foreground: Rgb888,
        font: &'static MonoFont<'static>,
    ) -> Self;
}
```

The namespace isolates framework-owned orientation, calibration, and simulated
platform state. It must be stable and unique per deployed application.

Do not add optional application services to this type. A clock setter, DNS
response, startup artwork, or application notice text is not CYD construction
configuration.

### `CydWebCommand`

The application returns a platform request rather than manipulating the
browser lifecycle itself:

```rust,no_run
pub enum CydWebCommand {
    Restart,
    Recalibrate,
    ResetWifi,
    Reorientate(Orientation),
    Stop,
}
```

The implemented names may be adjusted for clarity, but the semantics must stay
typed and application-neutral.

- `Restart` reconstructs the current virtual device without clearing persistent
  state.
- `Recalibrate` clears only touch calibration state, releases transient input,
  runs the standard browser calibration flow, and restarts the application.
- `ResetWifi` resets the shared simulated Wi-Fi state and restarts the
  application at its connection phase.
- `Reorientate` persists the requested orientation, reconstructs the canvas and
  virtual device in that orientation, and restarts the application.
- `Stop` ends the supervisor normally without producing a fatal notice.

The framework must wait for a fresh BOOT press after a BOOT-caused command. A
held browser button must not create an immediate restart loop. Implement this
with input state and explicit coordination, not a timer delay.

Application-specific exit enums remain in their core crates. Each `inner_main`
contains the short, exhaustive mapping from that enum to `CydWebCommand`.

### `start_cyd_web_app`

Provide a generic start function with the conceptual contract:

```rust,no_run
pub fn start_cyd_web_app<Run, Error>(
    canvas_id: &str,
    config: CydWebAppConfig,
    inner_main: Run,
) -> Result<CydWebAppHandle, JsValue>
where
    for<'a> Run: AsyncFnMut(
            &'a mut CydWasm,
            &'a mut ButtonWasm,
        ) -> Result<CydWebCommand, Error>
        + 'static,
    Error: Debug + 'static;
```

This signature is illustrative where compiler syntax requires adjustment. Its
observable contract is mandatory:

- synchronously find and validate the canvas so `start` can return an immediate
  setup error;
- create stable shared supervisor state and the JavaScript handle;
- spawn exactly one framework-owned supervisor task;
- construct a new simulator session before every call to `inner_main`;
- apply a returned `CydWebCommand` within the same supervisor;
- turn an application error into one typed fatal notice and stop;
- return the stable handle immediately after the first coherent input target
  and canvas orientation exist.

Do not require every application to define a `wasm_bindgen` class. Its only
required export should normally be the one-line `start` wrapper.

### `CydWebAppHandle`

Expose one standard `#[wasm_bindgen]` handle used by every application. It
proxies browser input to whichever simulator session is current:

```rust,no_run
#[wasm_bindgen]
pub struct CydWebAppHandle { /* shared runtime state */ }

#[wasm_bindgen]
impl CydWebAppHandle {
    pub fn touch_down(&self, position_x: f32, position_y: f32);
    pub fn touch_move(&self, position_x: f32, position_y: f32);
    pub fn touch_up(&self);
    pub fn boot_down(&self);
    pub fn boot_up(&self);
    pub fn orientation_is_inverted(&self) -> bool;
    pub fn take_notice(&self) -> Option<CydWebNotice>;
    pub fn request_restart(&self);
    pub fn clear_storage_and_restart(&self);
}
```

`request_restart` and `clear_storage_and_restart` are host/development
operations, not the normal way an application handles core exits. They must
signal the supervisor through a real lifecycle channel or signal. The
supervisor selects that signal against the running application future and
performs an orderly replacement. Do not poll a shared cell or sleep while
waiting for a request.

The handle must safely ignore input before the first session, during a session
swap, and after a fatal stop. It must release active touch and BOOT state before
dropping a session.

### Typed notices

Keep notices distinct from lifecycle commands. Expose a `CydWebNotice` value
with typed severity and a stable identifier. JavaScript maps identifiers to
localized presentation text. Do not return application lifecycle results as
strings such as `"orientation"`, `"idle"`, or `"runtime error"`.

At minimum, support the existing Wi-Fi and fatal-runtime notices. Notice
replacement and accessibility behavior remain owned by the shared browser
shell.

The framework may internally queue notices, but queue ownership and wake-up
must be centralized. Application crates must not own their own notice slots.

## Supervisor lifecycle

For each application session, the supervisor performs this sequence:

1. Load the framework state for the configured namespace.
2. Select the saved orientation or `initial_orientation`.
3. Construct `CydSimulatorWasm` with the configured style.
4. Atomically replace the live input-control target in `CydWebAppHandle`.
5. Complete calibration when calibration state is absent.
6. Call the application's `inner_main(&mut cyd, &mut button)`.
7. Select the application future against host lifecycle requests.
8. Release transient input and remove the old live target.
9. Apply the returned command or host request.
10. Reconstruct and run again, stop normally, or report a fatal error.

There must never be two live application tasks for one handle. Restart must not
be implemented by recursively calling exported `start`, and JavaScript must not
have to notice that Rust returned before a restart occurs.

The supervisor owns only CYD platform lifecycle. Startup screens, Wi-Fi status
rendering, DNS construction, clock construction, and the call into the generic
core remain visible in `inner_main`.

## Browser capability adapters

The framework alone is not enough to make the five launchers concise. Extract
the browser capabilities currently duplicated in application crates.

### `ClockSyncWasm`

Move the duplicate Linkage Blaze browser clock implementation into Device
Envoy's WASM support as `ClockSyncWasm`. It implements `ClockSync` using browser
wall-clock time and local UTC offset.

Provide a small control path for the optional page time setter. Keep this
separate from `CydWebAppConfig`; only clock applications export or configure a
time setter.

The Clock and Skeleton Clock launchers must not retain duplicate
`BrowserClockSync` implementations after migration.

### Simulated Wi-Fi

Replace the loose `simulate_wifi_connect` helper with a small constructed
browser adapter whose API reads like a platform resource. The exact name may be
`WifiAutoWasm` if it can implement the canonical `WifiAuto` contract cleanly;
otherwise use a precise name such as `WifiSimulatorWasm` and do not claim trait
conformance.

It must:

- emit canonical `WifiAutoEvent` values rather than a parallel WASM-only event
  enum where practical;
- use explicit state transitions and input coordination;
- support deterministic success and explicit failure injection for tests;
- expose reset behavior used by `CydWebCommand::ResetWifi`;
- avoid returning a fake hardware network stack when the browser does not have
  one.

Deliberate simulated connection pacing may use animation frames. Timing must not
be used to resolve ownership or restart races.

### Deterministic browser DNS

The browser cannot perform arbitrary DNS resolution through the ordinary web
platform. Provide an explicitly named deterministic `Dns` implementation for
demos and tests, such as `DnsFixedWasm`. Its constructor receives the addresses
it will return.

Do not call it a real browser DNS resolver and do not hide the substitution in
the framework. DNS Tester's `inner_main` should visibly construct it, making the
capability substitution honest and easy to explain.

## Target application launchers

The following examples describe the desired readability. They are design
targets, not code to copy mechanically if final error types require small
changes.

### Armatron

Armatron constructs no additional browser capability:

```rust,no_run
async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, ArmatronError<Infallible>> {
    match armatron(cyd, button).await? {
        ArmatronExit::CalibrationRequested => Ok(CydWebCommand::Recalibrate),
    }
}
```

Remove the app-owned infinite loop, `wait_for_button_release`, `spawn_local`,
and direct console error reporting.

### Ballet

Ballet asks the CYD only for its display capability:

```rust,no_run
async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, BalletError<Infallible>> {
    let mut display = cyd.display();
    match ballet(&mut display, button).await {
        Ok(never) => match never {},
        Err(error) => Err(error),
    }
}
```

This example demonstrates that the framework supplies a CYD platform without
forcing the generic application to consume touch.

### Clock and Skeleton Clock

Each clock launcher explicitly constructs `ClockSyncWasm`, renders its shared
splash, performs the browser Wi-Fi simulation, and calls its existing core
loop. The two launchers may share a small Linkage Blaze helper for their truly
identical startup policy, but the framework must not contain Linkage
Blaze-specific status rectangles or text.

The final flow should read approximately:

```rust,no_run
async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, MainError> {
    let mut display = cyd.display();
    let clock_sync = ClockSyncWasm::new();
    clock_splash(&mut display).await?;

    let wifi_simulator = WifiSimulatorWasm::new();
    wifi_simulator
        .connect(button, async |wifi_auto_event| {
            render_wifi_status(&mut display, wifi_auto_event).await
        })
        .await?;

    match clock(&mut display, &clock_sync, button).await? {
        ClockExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
    }
}
```

No clock launcher should classify simulator notices, manually wait for button
release, or own an outer restart loop.

### DNS Tester

DNS Tester should become an ordinary function-based launcher rather than a
custom `DnsTesterWeb` state machine:

```rust,no_run
async fn inner_main(
    cyd: &mut CydWasm,
    button: &mut ButtonWasm,
) -> Result<CydWebCommand, MainError> {
    dns_tester::splash(cyd).await?;

    let wifi_simulator = WifiSimulatorWasm::new();
    wifi_simulator
        .connect(button, async |wifi_auto_event| {
            dns_tester::wifi_status(cyd, wifi_auto_event).await?;
            Ok(())
        })
        .await?;

    let mut dns = DnsFixedWasm::new([IpAddress::Ipv4([127, 0, 0, 1].into())]);
    match dns_tester::run(cyd, button, &mut dns).await? {
        DnsTesterExit::Calibrate => Ok(CydWebCommand::Recalibrate),
        DnsTesterExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
        DnsTesterExit::Reorientate(orientation) => {
            Ok(CydWebCommand::Reorientate(orientation))
        }
    }
}
```

Remove `DnsTesterWeb`, its internal state cells, `take_exit`, `reboot`, manual
canvas resizing, duplicated orientation storage, and direct splash rendering.

## JavaScript API

The shared page bootstrap should remain declarative:

```javascript
import init, { start } from "./pkg/application.js";
import { mountCydSimulator } from "./cyd-simulator.js";

await init();
await mountCydSimulator({
  start,
  app: {
    title: "Armatron",
    previewLine: "Control a simulated linkage on a CYD.",
    descriptionHtml: "<p>...</p>",
    controlsHtml: "<p>...</p>",
    coreCodeUrl: "https://github.com/...",
    galleryUrl: "../../",
  },
});
```

`mountCydSimulator` calls `start("screen")`, binds the returned standard handle
once, synchronizes presentation from intrinsic canvas size and handle
orientation, and consumes typed notices.

The page must not:

- call application-specific `take_exit` or `reboot` methods;
- translate Rust exit strings;
- rebind input after an ordinary application restart;
- resize the canvas in response to application policy;
- decide whether calibration, Wi-Fi reset, or orientation requires restart;
- know whether the current Rust session has been reconstructed.

Optional page-only controls such as the clock time setter remain declarative
extensions in the shared shell. They call a narrow application export and do
not change the standard CYD lifecycle protocol.

## Error model

Keep setup errors and runtime errors distinct.

- Canvas lookup and initial simulator construction errors are returned directly
  from exported `start` as `JsValue`.
- Application runtime errors are formatted once by the Rust supervisor,
  submitted as a fatal typed notice, and stop the supervisor.
- Infallible CYD/display/touch branches must remain exhaustively matched without
  invented fallback errors.
- The shared shell presents fatal notices accessibly and does not silently
  restart a failed application.
- Tests may request an explicit restart after a fatal stop, but normal pages do
  not automatically conceal failures.

Application launchers should use ordinary `?` propagation into one local error
enum where multiple error sources exist. Do not make every launcher manually
convert errors to `JsValue` or log them to `console`.

## Ownership and async requirements

The implementation must be safe Rust and must not add `unsafe` blocks.

The stable handle may use shared interior state internally because JavaScript
and the supervisor own different ends of one long-lived browser device. That
interior state belongs in the framework, not in every application crate.

Use explicit asynchronous coordination for host lifecycle requests and session
replacement. Do not use timer sleeps, animation-frame counts, or shared-cell
polling to make a task appear to have stopped. Dropping a selected application
future during an explicit host restart must first detach its live input target
and release transient input.

Document cancellation assumptions for the supplied CYD, button, clock, Wi-Fi,
and application futures. Add tests that repeatedly request restart at each
startup phase to catch stale tasks and stale input targets.

## Migration plan

### Phase 1: framework core

- Add `CydWebAppConfig`, `CydWebCommand`, `CydWebAppHandle`, typed notice output,
  and `start_cyd_web_app` to Device Envoy WASM support.
- Implement the single-task supervisor and explicit host-command coordination.
- Keep `CydSimulatorWasm` as the lower-level constructor used by the
  supervisor.
- Unit-test command application, stable-handle replacement, persistent
  orientation, input release, fatal stop, and storage clearing.

### Phase 2: shared browser capabilities

- Extract `ClockSyncWasm` from the duplicated Linkage Blaze implementations.
- Replace the loose Wi-Fi helper and WASM-only event vocabulary with the
  constructed browser Wi-Fi adapter.
- Add the explicitly deterministic `DnsFixedWasm` implementation.
- Test each adapter independently through its canonical capability trait where
  one exists.

### Phase 3: migrate Linkage Blaze

- Migrate Ballet first as the smallest non-returning display application.
- Migrate Armatron and verify calibration/restart behavior.
- Migrate Clock and Skeleton Clock and delete their duplicated clock and Wi-Fi
  lifecycle code.
- Update the four page bootstraps to use one standard handle contract.
- Preserve existing screen rendering and page presentation.

### Phase 4: migrate DNS Tester

- Replace `DnsTesterWeb` with `WEB_APP`, exported `start`, and `inner_main`.
- Use the shared DNS Tester splash and Wi-Fi status helpers.
- Map all three DNS Tester exit variants exhaustively to framework commands.
- Remove the JavaScript exit-string polling and page-owned restart policy.
- Preserve orientation, calibration, BOOT, Wi-Fi simulation, and storage
  behavior covered by the current Playwright test.

### Phase 5: documentation and cleanup

- Make the new framework the primary documented path for a complete CYD WASM
  application.
- Keep low-level `CydSimulatorWasm` documentation as an advanced escape hatch.
- Delete obsolete app-specific lifecycle types and helpers; do not retain
  compatibility aliases.
- Update `CYD_WASM_SIMULATOR_SPEC.md` status and resolve any superseded API
  sketches.
- Only after implementation and validation, rewrite
  `CYD_WASM_CONSTRUCTION_MEDIUM_ARTICLE.md` around the final real code.

## Acceptance criteria

### Readability

- Each of the five WASM crates has one obvious `start` wrapper and one obvious
  `inner_main`-style async function.
- A reader can see every application-specific capability being constructed and
  passed to the generic core.
- Every core `Exit` variant is handled in one adjacent exhaustive match.
- None of the five application launchers directly uses `spawn_local`, `Rc`,
  `Cell`, `RefCell`, canvas DOM lookup, or direct simulator-control storage.
- None owns an outer restart loop or a button-release wait helper.
- DNS Tester has no application-specific exported wrapper class or string exit
  protocol.

### Behavior

- All five applications use the same stable JavaScript handle and shared shell.
- Touch and BOOT remain usable during startup and after any number of restarts.
- A held BOOT button cannot cause repeated exits.
- Orientation changes persist across restart and browser reload.
- Recalibration clears only calibration state and follows the standard browser
  calibration flow.
- Wi-Fi reset follows one shared simulated policy.
- Fatal errors produce one typed fatal notice and leave no live application
  task.
- Existing application rendering and golden output remain unchanged unless a
  separately identified bug requires correction.

### Testing

- Add Device Envoy unit tests for the supervisor state machine and browser
  capability adapters.
- Add or update WASM browser tests for typed notices and stable-handle behavior.
- Run the existing DNS Tester Playwright lifecycle test against the new
  framework without weakening its assertions.
- Run all Linkage Blaze Playwright examples, including clock time controls,
  BOOT behavior, touch behavior, and dynamic orientation presentation.
- Add a small sixth fixture application in tests to prove that adopting the
  framework requires no application-specific JavaScript lifecycle code.
- Run Device Envoy `cargo check-all`.
- Run Linkage Blaze `just check-all`.

## Medium article test

The final API passes the explanatory test when an article can honestly show:

1. the generic core signature;
2. the ESP or RP `inner_main` constructing hardware capabilities;
3. the WASM `inner_main` constructing browser capabilities;
4. the same core call in both;
5. one exhaustive exit-to-platform-command match in both;
6. a one-line WASM framework start call;
7. a declarative page mount with no hidden application state machine in
   JavaScript.

The article should be able to summarize the framework as:

> Construct the browser capabilities, run the generic application, and return
> the platform command. The CYD web supervisor applies it and runs the next
> coherent session.

If the article must explain app-owned cells, task spawning, polling, manual
canvas synchronization, or string exit messages, this specification has not
yet been satisfied.
