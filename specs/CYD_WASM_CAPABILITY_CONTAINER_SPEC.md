<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# CYD WASM Capability Container and Page UI

## Status

Planning only. This is the next refinement of the implemented CYD WASM
application framework. It applies to Device Envoy DNS Tester and the Linkage
Blaze Armatron, Ballet, Clock, and Skeleton Clock browser applications.

This specification takes precedence over the following decisions in
`CYD_WASM_APP_FRAMEWORK_SPEC.md`:

- providing different `start_cyd_web_app` and
  `start_cyd_display_web_app` callback shapes;
- passing `CydWasm` and `ButtonWasm` as separate callback arguments;
- requiring every optional browser capability to be constructed visibly in
  `inner_main`; and
- keeping application information and optional browser controls entirely in
  application-specific JavaScript.

The generic core boundary remains unchanged. Core functions must still ask for
focused `Cyd`, `CydDisplay`, `Button`, `ClockSync`, and `Dns` capabilities. The
unified container exists only at the WASM construction boundary.

Breaking changes to the unreleased WASM framework and generated browser pages
are allowed. Do not retain compatibility aliases, deprecated start functions,
or parallel JavaScript configuration paths.

## Objective

Make the complete browser port explainable from its Rust launcher without
creating a start function for every combination of optional capabilities.

The final teaching story must be:

1. `start_cyd_web_app` constructs one browser application environment.
2. The environment contains all standard capabilities supplied by the CYD
   browser framework.
3. `inner_main` selects the capabilities required by its unchanged generic
   core function.
4. Calling `ClockSyncWasm::show()` opts into the corresponding browser time
   control; it is hidden by default.
5. Rust supplies the text and links for the shared information panel.
6. JavaScript renders framework state and forwards user input, but contains no
   application-specific capability policy.

The framework must not encode capability combinations in function names. In
particular, do not add APIs such as `start_cyd_clock_web_app`,
`start_cyd_dns_web_app`, or any other combinatorial start-function family.

## Architecture

```text
Rust start function
  CydWebAppConfig + CydWebPageInfo + inner_main
                         |
                         v
               one CYD web supervisor
                         |
                         v
          one owned CydWebAppWasm per run
  CydWasm + ButtonWasm + ClockSyncWasm + browser simulators
                         |
                         v
            app-specific inner_main
       select capabilities and call generic core
                         |
                         v
                 generic core function
       sees only Cyd / CydDisplay / Button / ClockSync / Dns

stable CydWebAppHandle <----> shared JavaScript page shell
  pointer + BOOT input         information panel
  lifecycle commands           optional time control
  typed notices                notices and device chrome
```

The capability container is not a replacement for focused core traits. It is
the owned set of browser resources from which a short platform constructor can
choose those traits.

## One start function

Keep one public application entry point:

```rust,no_run
pub fn start_cyd_web_app<Run, Error>(
    canvas_id: &str,
    config: CydWebAppConfig,
    page_info: CydWebPageInfo,
    inner_main: Run,
) -> Result<CydWebAppHandle, JsValue>
where
    Run: AsyncFnMut(CydWebAppWasm) -> Result<CydWebCommand, Error> + 'static,
    Error: Debug + 'static;
```

The precise stable-Rust spelling may change if required by `AsyncFnMut`, but
`inner_main` must receive one owned environment rather than a variable-arity
list of capabilities.

Remove `start_cyd_display_web_app`. Display-only applications use the same
supervisor and explicitly narrow `CydWasm` to `CydDisplayWasm` with
`CydWasm::display()`.

The supervisor constructs a new `CydWebAppWasm` for every run and passes it to
`inner_main` by value. A restart or lifecycle interruption drops that complete
run and constructs a fresh environment. Do not make application code manage
`Rc`, `RefCell`, task cancellation, or resource replacement.

## The browser capability container

Provide one concrete browser-only resource type. The preferred name is
`CydWebAppWasm`:

```rust,no_run
pub struct CydWebAppWasm {
    pub cyd: CydWasm,
    pub button: ButtonWasm,
    pub clock_sync: ClockSyncWasm,
    pub wifi_simulator: WifiSimulatorWasm,
    pub dns_simulator: DnsSimulatorWasm,
}
```

The exact field visibility may be replaced with one non-combinatorial
`into_parts()` operation if implementation invariants require it. Do not add
accessors such as `cyd_and_button`, `display_button_clock`, or other methods
that recreate the capability-combination problem.

These resources are deliberately available to every CYD browser launcher.
Unused resources are ordinary ignored fields, not a reason to add a new start
function. Their construction must be cheap and must not display UI, start
network activity, or change persistent state merely because they exist.

`ClockSyncWasm` must be available under the `wasm` feature without requiring
an unrelated `wifi` feature. Move its module gating accordingly. This change
must not enable the WASM Embassy time driver for any MCU target beyond the
already opt-in `wasm` feature.

### Display capability narrowing

Keep and use the existing non-consuming method:

```rust,no_run
let mut display = cyd_web_app_wasm.cyd.display();
```

`CydWasm::display()` returns a `CydDisplayWasm` sharing the same browser canvas
state. It is the correct spelling because the operation selects a display
capability without destroying the simulated CYD.

Do not add `into_display()`. That name would promise a consuming ownership
conversion and would unnecessarily discard touch capability owned by the
current browser run.

Display-only generic core code must receive only `&mut CydDisplayWasm`. It must
not receive `&mut CydWasm` merely because the containing WASM launcher owns one.

## Rust-owned page information

Move semantic information-panel content out of application-specific
JavaScript. Add a small copyable value with a direct constructor:

```rust,no_run
pub struct CydWebPageInfo {
    pub title: &'static str,
    pub preview: &'static str,
    pub description: &'static str,
    pub controls: &'static str,
    pub core_code_url: &'static str,
}

impl CydWebPageInfo {
    pub const fn new(
        title: &'static str,
        preview: &'static str,
        description: &'static str,
        controls: &'static str,
        core_code_url: &'static str,
    ) -> Self;
}
```

Keep `CydWebPageInfo` separate from `CydWebAppConfig`:

- `CydWebAppConfig` constructs the simulated device and identifies its
  persistent state namespace.
- `CydWebPageInfo` describes the explanatory page surrounding that device.

Use plain text for `preview`, `description`, and `controls`. The shared shell
must escape and render this content rather than accepting arbitrary per-app
HTML. Static shared headings and link presentation remain JavaScript-shell
concerns.

Deployment-specific values such as a relative gallery URL, service-worker
path, or versioned asset directory may remain host-page configuration. The
application title, explanation, controls, and core-code link must not be
duplicated in JavaScript.

The stable handle must expose the page information to the shell through typed
getters or one typed page-information object. Do not serialize it through an
ad hoc delimiter-separated string.

## Optional browser UI owned by capabilities

Constructing a capability must not automatically display its optional host UI.
The capability itself requests that UI when the application uses it.

### Clock control

`ClockSyncWasm` must default to live browser-local time with its browser time
control hidden. Add:

```rust,no_run
impl ClockSyncWasm {
    pub fn show(&self);
}
```

Calling `show()` requests the shared shell to display the time chip, slider,
readout, and **Live** button for this application. Repeated calls are
idempotent.

The control must operate on the same logical `ClockSyncWasm` used by the core:

- selecting a time changes that clock's time-of-day override;
- **Live** clears the override and resumes browser-local wall-clock time;
- the selected override remains effective across ticks;
- hiding or omitting the control leaves the clock live; and
- one application's clock state must not affect another application namespace
  or browser instance.

Replace the process-global `SELECTED_TIME_OF_DAY` state with state owned by the
browser application/clock capability and shared with `CydWebAppHandle`. Do not
retain an application-specific exported `set_time_of_day` free function.

The handle may expose typed methods used by the shell, for example:

```rust,no_run
pub fn clock_control_is_visible(&self) -> bool;
pub fn set_clock_time_of_day(&self, seconds_of_day: u32) -> Result<(), JsValue>;
pub fn use_live_clock(&self);
```

Equivalent typed names are acceptable. Invalid time-of-day values must return
an error; do not clamp them. The JavaScript shell must observe visibility even
when `show()` runs asynchronously after `start_cyd_web_app` has returned.

Do not put `show_clock_control: bool` in `CydWebAppConfig`. The request belongs
next to the capability use in `inner_main`.

Future UI-supported capabilities should follow this same pattern: one standard
capability in `CydWebAppWasm`, hidden host UI by default, and an idempotent
capability method that requests its UI. They must not create additional start
functions.

## Cleaner simulated DNS

The DNS Tester launcher must not construct an address vector or expose the
loopback representation merely to obtain the framework's standard browser DNS
simulation.

Replace the application-facing `DnsFixedWasm` story with
`DnsSimulatorWasm`, constructed as part of `CydWebAppWasm`. Its standard
framework configuration must:

- implement the existing generic `Dns` trait;
- return the deterministic IPv4 loopback address for every hostname;
- wait for a real minimum of 12 ms at the capability boundary;
- remain fallible only as required by the trait implementation; and
- perform no browser request merely because it was constructed.

The DNS Tester then passes the provided simulator directly:

```rust,no_run
dns_tester::run(
    &mut cyd_web_app_wasm.cyd,
    &mut cyd_web_app_wasm.button,
    &mut cyd_web_app_wasm.dns_simulator,
)
.await
```

If tests or low-level callers need configurable fixed addresses or latency,
provide one direct `DnsSimulatorWasm::new(addresses, latency)` constructor.
Application launchers must use the standard instance supplied by
`CydWebAppWasm`.

Do not make an HTTP fetch and label its end-to-end duration as DNS latency. A
future browser `HttpProbe` capability may expose real resource timing, but it
would not return the operating system resolver's addresses and is outside this
change.

## Target launcher shapes

The examples below describe the intended application-facing structure. Use
the real current error and exit types during implementation.

### Armatron

```rust,no_run
async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<
    CydWebCommand,
    linkage_blaze_core::examples::armatron::Error<Infallible>,
> {
    match armatron(&mut cyd_web_app_wasm.cyd, &mut cyd_web_app_wasm.button).await? {
        ArmatronExit::CalibrationRequested => Ok(CydWebCommand::CalibrationNotNeeded),
    }
}
```

### Ballet

```rust,no_run
async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<
    CydWebCommand,
    linkage_blaze_core::examples::ballet::Error<Infallible>,
> {
    let mut display = cyd_web_app_wasm.cyd.display();
    match ballet(&mut display, &mut cyd_web_app_wasm.button).await {
        Ok(never) => match never {},
        Err(error) => Err(error),
    }
}
```

### Clock and Skeleton Clock

```rust,no_run
async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<
    CydWebCommand,
    linkage_blaze_core::examples::clock::Error<Infallible>,
> {
    cyd_web_app_wasm.clock_sync.show();
    let mut display = cyd_web_app_wasm.cyd.display();

    // Existing splash and simulated Wi-Fi presentation remain here.

    match clock(
        &mut display,
        &cyd_web_app_wasm.clock_sync,
        &mut cyd_web_app_wasm.button,
    )
    .await?
    {
        Exit::ResetWifi => Ok(CydWebCommand::ResetWifi),
    }
}
```

Skeleton Clock has the same capability construction shape and calls its own
generic core function.

### DNS Tester

```rust,no_run
async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<CydWebCommand, CoreError<Infallible, Infallible>> {
    dns_tester::splash(&mut cyd_web_app_wasm.cyd).await?;

    // Existing simulated Wi-Fi connection and status presentation remain here,
    // using cyd_web_app_wasm.wifi_simulator and cyd_web_app_wasm.button.

    match dns_tester::run(
        &mut cyd_web_app_wasm.cyd,
        &mut cyd_web_app_wasm.button,
        &mut cyd_web_app_wasm.dns_simulator,
    )
    .await?
    {
        CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded),
        CoreExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
        CoreExit::Reorientate(orientation) => {
            Ok(CydWebCommand::Reorientate(orientation))
        }
    }
}
```

## JavaScript shell migration

The shared shell must become the renderer of Rust-supplied application
information and capability-requested controls.

Remove per-application JavaScript fields for:

- `title`;
- `previewLine`;
- `descriptionHtml`;
- `controlsHtml`;
- `coreCodeUrl`; and
- `timeSetter`.

Clock and Skeleton Clock pages must stop importing `set_time_of_day`. The shell
uses the stable handle's typed clock-control methods when `ClockSyncWasm::show()`
requests that UI.

Keep page-host concerns such as WASM initialization, the canvas element,
gallery navigation, service-worker registration, and optional case-alignment
diagnostics in JavaScript.

Update the shared shell source first, then regenerate or copy it through the
repositories' established browser-page generation process. Do not patch only
one generated deployment copy.

## Tests

### Rust/WASM tests

- One `start_cyd_web_app` callback receives a complete `CydWebAppWasm`.
- A touch core can use its `CydWasm` and button.
- A display-only core obtains `CydDisplayWasm` through `CydWasm::display()` and
  runs without consuming the CYD.
- `ClockSyncWasm` starts live with its control hidden.
- `ClockSyncWasm::show()` is idempotent and becomes observable through the
  stable handle after asynchronous startup.
- Setting a valid time through the handle changes the same clock observed by
  the generic core.
- Returning to Live resumes browser-local time.
- Invalid seconds-of-day values return an error rather than being clamped.
- Clock override and visibility state are isolated between application
  instances and namespaces.
- `DnsSimulatorWasm` returns loopback only after at least 12 ms of Embassy time.
- A lifecycle interruption drops the whole owned capability container and a
  restart receives fresh resources with the appropriate persistent state.

### Browser tests

- Every application displays its Rust-supplied title, description, controls,
  and core-code link in the information panel.
- Application-specific JavaScript contains no duplicate semantic information
  text.
- Armatron and DNS Tester receive browser touch through `CydWasm`.
- Ballet renders through the display returned by `CydWasm::display()` and shows
  no calibration or time UI.
- Clock and Skeleton Clock show the time control only after calling
  `ClockSyncWasm::show()`.
- Moving either clock slider changes the corresponding simulated clock; Live
  returns it to browser time.
- Armatron, Ballet, and DNS Tester do not show the time control.
- DNS Tester reports a measured simulated latency of at least 12 ms and never
  performs a hidden HTTP request.
- Reorientation and restart preserve a stable JavaScript handle and do not
  duplicate information panels or optional controls.

## Documentation

After implementation, revise `CYD_WASM_FRAMEWORK_MEDIUM_ARTICLE.md` around the
new complete construction story. It must show real code for:

- the single Rust `start` function with `CydWebPageInfo`;
- the owned `CydWebAppWasm` callback;
- display narrowing with `.display()`;
- `ClockSyncWasm::show()` and the resulting browser time control;
- the Rust-owned information-panel text; and
- the provided deterministic `DnsSimulatorWasm`.

The article must distinguish device capabilities from host-page presentation,
while showing that both are declared from Rust and rendered by the shared
framework.

Update `CYD_WASM_APP_FRAMEWORK_SPEC.md` after implementation so its description
of the final API no longer teaches the superseded dual-entry-point design.
Keep `CYD_WASM_BROWSER_POLICY_FIX_SPEC.md` authoritative for the no-browser-
calibration policy.

## Validation

After implementation:

1. Run Device Envoy core and DNS Tester WASM tests and checks.
2. Regenerate the DNS Tester WASM package and browser files.
3. Build and test all four Linkage Blaze WASM applications.
4. Regenerate the affected Linkage Blaze browser pages through their source-of-
   truth process.
5. Run the browser suites for DNS Tester, Armatron, Ballet, Clock, and Skeleton
   Clock.
6. Run Device Envoy `cargo check-all` and Linkage Blaze `just check-all`.
7. Run `git diff --check` in both repositories.

The existing Ballet long-running const-evaluation warning is outside this
change and must not be suppressed.

## Acceptance criteria

- There is one CYD WASM start function, independent of capability combinations.
- Each supervisor run owns one complete browser capability container.
- Generic core functions remain capability-specific and unchanged.
- Display-only launchers use `.display()`; no `into_display()` API exists.
- Clock UI is hidden by default and requested with `ClockSyncWasm::show()`.
- Clock UI state belongs to the corresponding application rather than a
  process-global static.
- Information-panel semantic content comes from Rust and is not duplicated in
  per-application JavaScript.
- DNS Tester uses the framework-provided `DnsSimulatorWasm` without spelling a
  loopback address in its launcher.
- Simulated DNS remains deterministic and waits at least 12 ms.
- No new start-function family, builder, macro language, boxed application
  trait, or untyped string protocol is introduced.
- Browser calibration behavior remains absent and unchanged.
- ESP, RP, memory, and platform-neutral generic core behavior remain unchanged.
