<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# CYD Web Namespace API

## Status

Planning only. This specification describes a naming and module-organization
refinement of the implemented CYD browser application framework.

The runtime architecture is already correct. This change must preserve its
behavior while making the Rust API and the five example launchers easier to
read and explain.

Breaking changes to this unreleased Rust API are allowed. Do not retain
compatibility aliases, deprecated names, duplicate re-exports, or parallel
module paths.

The hypothetical Medium articles are non-authoritative drafts and are outside
this migration. Do not edit them as part of this work. They may receive a
separate editorial update after the API has settled.

## Objective

Replace repeated `CydWeb...` prefixes with one descriptive Rust namespace and
give the value passed to `inner_main` a name that matches its role.

The target launcher signature is:

```rust,no_run
async fn inner_main(
    mut capabilities: cyd_web::Capabilities,
) -> Result<cyd_web::Command, SkeletonClockError<Infallible>> {
    // Select capabilities and call the unchanged generic core.
}
```

The name relationship is intentional:

```rust,no_run
capabilities: cyd_web::Capabilities
```

The variable uses the complete unqualified type name converted to snake case.
The `cyd_web` module supplies shared context once instead of encoding it in
every public item name.

## Why a namespace

The current API repeats the same qualification across every framework item:

```rust,no_run
CydWebAppConfig
CydWebPageInfo
CydWebAppWasm
CydWebCommand
CydWebAppHandle
CydWebNotice
CydWebNoticeSeverity
start_cyd_web_app
```

That repetition indicates a missing module boundary. Rust modules are the
ordinary mechanism for expressing this relationship.

The final API must read:

```rust,no_run
cyd_web::Config
cyd_web::PageInfo
cyd_web::Capabilities
cyd_web::Command
cyd_web::Handle
cyd_web::Notice
cyd_web::NoticeSeverity
cyd_web::start
```

This also leaves a natural naming family for future browser simulators without
creating one start function per capability combination:

```rust,no_run
cyd_web::start(...)
led4_web::start(...)
led_strip_web::start(...)
```

Those future modules are illustrative only. Do not introduce them in this
change, and do not extract a speculative cross-platform web supervisor.

## Canonical module

Rename the framework module from `device_envoy_core::wasm::app` to:

```rust,no_run
device_envoy_core::wasm::cyd_web
```

Use the repository's file-module convention:

```text
crates/device-envoy-core/src/wasm.rs
crates/device-envoy-core/src/wasm/cyd_web.rs
```

Do not create a `mod.rs` file.

`wasm.rs` must expose the module directly:

```rust,no_run
pub mod cyd_web;
```

Remove the old `app` module and remove the root-level re-exports of its public
items. The canonical API must require qualification through `cyd_web`:

```rust,no_run
use device_envoy_core::wasm::cyd_web;
```

Continue to expose concrete device adapters such as `CydWasm`,
`ClockSyncWasm`, `DnsSimulatorWasm`, and `WifiSimulatorWasm` from their current
canonical locations. This specification changes the application-framework
namespace, not the device-adapter API.

## Public names

Apply these direct renames:

| Current name | Final name |
| --- | --- |
| `CydWebAppConfig` | `cyd_web::Config` |
| `CydWebPageInfo` | `cyd_web::PageInfo` |
| `CydWebAppWasm` | `cyd_web::Capabilities` |
| `CydWebCommand` | `cyd_web::Command` |
| `CydWebAppHandle` | `cyd_web::Handle` |
| `CydWebNotice` | `cyd_web::Notice` |
| `CydWebNoticeSeverity` | `cyd_web::NoticeSeverity` |
| `start_cyd_web_app` | `cyd_web::start` |

Do not add aliases from the old names. Do not re-export the short names from
`wasm`; names such as `Command`, `Config`, and `Handle` are intentionally clear
only when qualified by `cyd_web`.

## Capabilities

`Capabilities` owns the complete set of standard browser capabilities supplied
to one invocation of `inner_main`:

```rust,no_run
pub struct Capabilities {
    pub cyd: CydWasm,
    pub button: ButtonWasm,
    pub clock_sync: ClockSyncWasm,
    pub wifi_simulator: WifiSimulatorWasm,
    pub dns_simulator: DnsSimulatorWasm,
}
```

Use the plural name because this value is a collection of focused
capabilities. It is not itself a new generic capability and does not replace
the existing `Cyd`, `CydDisplay`, `Button`, `ClockSync`, or `Dns` traits.

Do not call the type `App`. The application is the `inner_main` function and
the generic core it invokes; `Capabilities` is what the browser platform gives
that application.

Do not include `Wasm` in this container name. The containing `cyd_web` module
already identifies the browser platform, while concrete adapter fields retain
their useful `Wasm` suffixes.

The supervisor remains the only constructor of `Capabilities`. Do not add a
public `Capabilities::new`, builder, combination-specific accessor, or
`into_parts` operation merely for this rename. Public fields remain the direct
way for `inner_main` to select capabilities.

Every supervisor restart must continue to construct a fresh `Capabilities`
value. Stable host state remains owned through `Handle` and the supervisor as
it is today.

## Start function

Move and rename the existing free function without changing its behavior:

```rust,no_run
pub fn start<Run, Error>(
    canvas_id: &str,
    config: Config,
    page_info: PageInfo,
    inner_main: Run,
) -> Result<Handle, JsValue>
where
    Run: AsyncFnMut(Capabilities) -> Result<Command, Error> + 'static,
    Error: Debug + 'static;
```

Keep this as a free function in the `cyd_web` namespace. Do not replace it
with:

- `Capabilities::new`;
- `Capabilities::new_and_start`;
- `Handle::start`;
- an automatic `#[wasm_bindgen(start)]` initializer;
- a macro that generates the application launcher; or
- a builder.

`cyd_web::start` starts the supervisor and returns a stable `Handle`; it does
not construct and return one `Capabilities` value. The free function states
that ownership accurately.

The application crate must retain one concrete WASM export because
`wasm-bindgen` cannot export the generic Rust framework function directly:

```rust,no_run
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<cyd_web::Handle, JsValue> {
    cyd_web::start(canvas_id, CONFIG, PAGE_INFO, inner_main)
}
```

Keep `canvas_id`; do not hard-code `"screen"` in Rust. The host page owns its
DOM element naming.

## Other public types

Keep the existing responsibilities and data of the renamed types:

- `Config` contains the storage namespace, initial orientation, display colors,
  and font.
- `PageInfo` contains Rust-owned title, preview, description, controls, and the
  core-code URL.
- `Command` contains the exhaustive application-to-supervisor requests.
- `Handle` is the stable JavaScript-facing input, lifecycle, notice, page-info,
  and optional-control interface.
- `Notice` and `NoticeSeverity` provide the typed browser-notice protocol.

Renaming must not change storage keys, notice IDs, command behavior, orientation
semantics, simulated Wi-Fi state, clock state, DNS behavior, or restart
ownership.

Where `wasm-bindgen` would otherwise expose overly generic JavaScript class or
enum names such as `Handle`, preserve the existing descriptive JavaScript names
with explicit `js_name` attributes. Rust callers should see the short
namespaced names; generated JavaScript and TypeScript should continue to use
unambiguous CYD web names.

## Launcher form

All five launchers must import the namespace rather than a list of prefixed
framework items:

```rust,no_run
use device_envoy_core::wasm::cyd_web;
```

Each launcher may import other concrete adapters or outcomes, such as
`WifiConnectOutcome`, when its implementation actually uses them.

Application-specific errors should be imported under concise, descriptive
names instead of appearing as fully qualified paths in `inner_main`
signatures. For example:

```rust,no_run
use linkage_blaze_core::examples::skeleton_clock::{
    Error as SkeletonClockError, Exit, skeleton_clock,
};
```

The representative Skeleton Clock boundary is:

```rust,no_run
const CONFIG: cyd_web::Config = cyd_web::Config::new(
    "linkage-blaze/skeleton-clock",
    ORIENTATION,
    BACKGROUND,
    FOREGROUND,
    &TOP_FONT,
);

const PAGE_INFO: cyd_web::PageInfo = cyd_web::PageInfo::new(
    "Skeleton Clock",
    "A motion-captured figure holds the hour and minute on placards.",
    "A clock told by a motion-captured figure whose placards show the hour and minute.",
    "It follows your local clock. Use the shared time control to scrub to any time of day.",
    "https://github.com/CarlKCarlK/linkage-blaze/blob/main/crates/linkage-blaze-core/src/examples/skeleton_clock.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<cyd_web::Handle, wasm_bindgen::JsValue> {
    cyd_web::start(canvas_id, CONFIG, PAGE_INFO, inner_main)
}

async fn inner_main(
    mut capabilities: cyd_web::Capabilities,
) -> Result<cyd_web::Command, SkeletonClockError<Infallible>> {
    capabilities.clock_sync.show();
    let mut display = capabilities.cyd.display();

    // Existing splash and simulated Wi-Fi presentation remain here.

    match skeleton_clock(
        &mut display,
        &capabilities.clock_sync,
        &mut capabilities.button,
    )
    .await?
    {
        Exit::ResetWifi => Ok(cyd_web::Command::ResetWifi),
    }
}
```

The comment abbreviates unchanged application-specific setup; the implemented
launcher must retain that setup and its error handling.

Apply the same shape to:

- Linkage Blaze Armatron;
- Linkage Blaze Ballet;
- Linkage Blaze Clock;
- Linkage Blaze Skeleton Clock; and
- Device Envoy DNS Tester.

Each launcher must use `capabilities` as the local name for
`cyd_web::Capabilities`. Do not abbreviate it to `app`, `context`, `caps`, or a
name that omits the type's meaning.

Generic core signatures and exit enums remain unchanged. Each `inner_main`
continues to narrow capabilities and map its core exit immediately to
`cyd_web::Command`.

## JavaScript boundary

Application JavaScript must continue to import its crate-local exported
`start` function. The shared shell API remains unchanged:

```javascript
import init, { start } from "./pkg/application.js";
import { mountCydSimulator } from "./cyd-simulator.js";

await mountCydSimulator({
  wasm: { init, start },
  app: { galleryUrl: "../../" },
});
```

Do not teach JavaScript about Rust's `cyd_web` module. Rust module namespaces
do not exist as runtime JavaScript namespaces after `wasm-bindgen` generation.

The returned handle must continue to provide the same callable JavaScript
methods and behavior. No browser page should need lifecycle, input, or notice
changes merely because the Rust API was renamed.

## Documentation

Add module documentation to `cyd_web` that explains the ownership boundary:

1. `start` creates the stable supervisor and returns `Handle`.
2. The supervisor constructs fresh `Capabilities` for each run.
3. `inner_main` selects focused capabilities for unchanged generic core code.
4. `Command` communicates platform policy back to the supervisor.

Put one complete compilable `rust,no_run` launcher example on
`Capabilities`. Other public types and methods should link to that example
rather than duplicating it.

Update ordinary rustdoc, code comments, and tests to use the final names. As
stated in Status, do not update the two hypothetical Medium articles during
this migration.

## Tests

Update existing Device Envoy WASM tests to use the namespace and final names.
Retain coverage for:

- receiving the complete `Capabilities` value;
- display narrowing through `capabilities.cyd.display()`;
- touch-capable use of `capabilities.cyd`;
- clock-control visibility and Live behavior;
- restart-persistent clock state;
- page information exposed through `Handle`;
- simulated DNS latency;
- orientation persistence and reconstruction;
- calibration-not-needed policy;
- simulated Wi-Fi reset; and
- fatal notice handling.

Browser tests must continue to pass without application-JavaScript changes
other than regenerated binding imports if generation requires them.

Add a compile-time or rustdoc example that demonstrates the final canonical
surface and prevents accidental root-level re-export drift:

```rust,no_run
use device_envoy_core::wasm::cyd_web;

fn receives_capabilities(_capabilities: cyd_web::Capabilities) {}
```

Do not add negative compile tests solely to prove that old names are absent.
Repository-wide search and the absence of aliases are sufficient.

## Generated artifacts

After implementation:

1. Regenerate the DNS Tester WASM package and browser deployment.
2. Rebuild the four Linkage Blaze WASM packages and affected browser pages
   through their existing source-of-truth commands.
3. Confirm generated JavaScript and TypeScript contain no stale Rust export
   names unless preserved intentionally through `wasm_bindgen(js_name = ...)`.
4. Do not hand-edit generated bindings as the source of the rename.

## Validation

Run, at minimum:

1. Device Envoy core WASM tests and checks with and without the `wifi` feature.
2. Device Envoy DNS Tester WASM checks and browser tests.
3. All four Linkage Blaze WASM checks.
4. The Linkage Blaze CYD browser suite, including DNS Tester.
5. Device Envoy `cargo check-all`.
6. Linkage Blaze `just check-all`.
7. `cargo fmt --all -- --check` in both repositories.
8. `git diff --check` in both repositories.
9. A repository-wide search proving the removed names and module path have no
   remaining code references outside the intentionally unchanged hypothetical
   articles.

The existing Ballet long-running const-evaluation warning is outside this
change and must not be suppressed.

## Acceptance criteria

- The canonical framework namespace is `device_envoy_core::wasm::cyd_web`.
- The public Rust surface is `Config`, `PageInfo`, `Capabilities`, `Command`,
  `Handle`, `Notice`, `NoticeSeverity`, and `start` within that namespace.
- No old `CydWeb...` Rust type, `start_cyd_web_app`, public `wasm::app` module,
  compatibility alias, or duplicate root re-export remains.
- Every `inner_main` receives
  `capabilities: cyd_web::Capabilities`.
- All five launchers use qualified `cyd_web` names and concise imported core
  error names.
- The concrete exported application `start` functions remain small and return
  `cyd_web::Handle`.
- Generic core functions and their capability bounds are unchanged.
- Runtime behavior, persistent browser state, JavaScript handle behavior,
  notices, clock controls, DNS simulation, touch mapping, orientation, and
  restart semantics are unchanged.
- No builder, macro launcher, automatic WASM initializer, or capability-
  combination start-function family is introduced.
- Generated artifacts and both repositories' complete checks pass.

## Out of scope

- Editing the hypothetical Medium articles.
- Adding another simulated hardware platform.
- Generalizing common code for hypothetical `led4_web` or `led_strip_web`
  modules.
- Changing the generic core capability traits.
- Changing browser calibration, DNS, Wi-Fi, clock, or orientation policy.
- Simplifying the application-specific Wi-Fi presentation in Clock or Skeleton
  Clock beyond the direct naming migration.
