<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# A CYD App in the Browser Should Still Look Like a CYD App

*A hypothetical Medium article about capability-based WASM construction for
ESP-like CYD applications.*

There is a tempting way to bring an embedded application to a web page: rewrite
the application around DOM callbacks, browser state, and JavaScript timers.
That works, but it loses the useful part of the embedded design: the app no
longer says what hardware-like capabilities it needs.

Device Envoy takes the opposite route. The browser supplies equivalents of the
capabilities that the core app already uses. The application stays an embedded
application. Its WASM launcher is just a small, readable construction boundary.

The result is not “the web version” and “the ESP version” of an app. It is one
core app with two motivated constructors.

## Start with the app, not the browser

The DNS Tester core loop asks for exactly three things:

```rust,no_run
pub async fn run<CydDevice, ButtonDevice, DnsDevice>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    dns: &mut DnsDevice,
) -> Result<Exit, Error<CydDevice::Error, DnsDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    DnsDevice: Dns,
{
    run_inner(cyd, button, dns).await
}
```

That signature is the important design decision. There is no SPI bus, HTML
canvas, browser storage, Wi-Fi stack, or JavaScript object in it. The core can
run on an ESP, RP, a deterministic memory device, or a browser simulator because
each platform can supply a `Cyd`, `Button`, and `Dns`.

When the app needs a platform decision, it returns one rather than taking one:

```rust,no_run
pub enum Exit {
    Calibrate,
    ResetWifi,
    Reorientate(Orientation),
}
```

The core says what happened. The platform decides what that means.

## ESP: construct physical capabilities

On an ESP, a constructor has real hardware work to do. It loads orientation,
creates display and touch devices, owns physical touch calibration, and returns
a ready-to-use `CydEsp`.

For example, the real generic ESP32 DNS Tester constructs its two-SPI CYD like
this:

```rust,no_run
let [
    wifi_auto_flash_block,
    mut calibration_flash_block,
    mut orientation_flash_block,
] = FlashBlockEsp::new_array::<3>(p.FLASH)?;
let orientation = orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);

static CYD_STATIC: CydStaticEsp<{ dns_tester::FRAME_PIXEL_COUNT }> = CydEsp::new_static();
let button = ButtonWatch::new(p.GPIO0, PressedTo::Ground, spawner).await?;

let mut cyd = CydEsp::new(
    &CYD_STATIC,
    p.SPI2,
    p.GPIO14,
    p.GPIO13,
    p.GPIO12,
    p.GPIO15,
    p.GPIO2,
    p.GPIO4,
    p.GPIO21,
    DEFAULT_DISPLAY_SPI_HZ,
    orientation,
    Rgb888::new(10, 10, 12),
    Rgb888::new(230, 230, 230),
    &DEFAULT_FONT,
    p.SPI3,
    p.GPIO25,
    p.GPIO32,
    p.GPIO39,
    p.GPIO33,
    p.GPIO36,
    &mut calibration_flash_block,
    &mut *button,
)
.await?;
```

That is not incidental complexity. Physical touch hardware has unknown raw
coordinates, so calibration and its persistent configuration belong at the
physical construction boundary.

Afterward, the rest is pleasantly ordinary:

```rust,no_run
let mut dns = DnsWithStack::new(*stack);

match dns_tester::run(&mut cyd, &mut *button, &mut dns).await? {
    CoreExit::Calibrate => {
        calibration_flash_block.clear()?;
    }
    CoreExit::ResetWifi => {
        wifi_auto.reset_to_captive_portal()?;
    }
    CoreExit::Reorientate(next_orientation) => {
        orientation_flash_block.save(&next_orientation)?;
    }
}
```

The returned enum keeps the policy readable. Clearing physical calibration is a
reasonable response on the physical CYD. It would be strange in a browser.

## WASM: construct browser capabilities

The browser has different facts, and the constructor should reflect them.

An HTML canvas already has known logical coordinates. Pointer events become
CYD touch coordinates through the simulator, so browser touch is intrinsically
usable. There is no physical touch controller to calibrate, no calibration
screen, and no calibration storage.

The shared framework owns the mechanical browser work:

- finding the canvas by ID;
- sizing it for the selected orientation;
- constructing a simulated CYD or display;
- forwarding pointer and BOOT events through a stable handle;
- persisting orientation;
- restarting one application task safely; and
- delivering typed notices to the shared JavaScript shell.

That leaves an application crate with two visible pieces: a small configuration
and a small `inner_main`.

## The DNS Tester launcher

Here is the complete browser-facing shape of the DNS Tester:

```rust,no_run
const WEB_APP: CydWebAppConfig = CydWebAppConfig::new(
    "device-envoy/dns-tester",
    Orientation::Landscape,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
);
const PAGE_INFO: CydWebPageInfo = CydWebPageInfo::new(
    "DNS Tester",
    "Measure a deterministic simulated DNS lookup on a CYD.",
    "The DNS tester exercises the shared device abstraction and reports a fixed browser simulation result.",
    "Touch the panel and press BOOT to interact with the tester.",
    "https://github.com/CarlKCarlK/device-envoy/blob/main/crates/device-envoy-examples-core/src/dns_tester.rs",
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, wasm_bindgen::JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}
```

`WEB_APP` says only how the application is presented and where its framework
state belongs. It is intentionally not a builder and does not smuggle DNS,
clock, or page-specific policy into the framework.

The app-specific constructor is just as direct:

```rust,no_run
async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<CydWebCommand, CoreError<Infallible, Infallible>> {
    dns_tester::splash(&mut cyd_web_app_wasm.cyd).await?;

    if matches!(
        cyd_web_app_wasm
            .wifi_simulator
            .connect(&mut cyd_web_app_wasm.button, async |wifi_auto_event| {
                dns_tester::wifi_status(&mut cyd_web_app_wasm.cyd, wifi_auto_event).await
            })
            .await?,
        WifiConnectOutcome::ResetRequested
    ) {
        return Ok(CydWebCommand::ResetWifi);
    }

    match dns_tester::run(
        &mut cyd_web_app_wasm.cyd,
        &mut cyd_web_app_wasm.button,
        &mut cyd_web_app_wasm.dns_simulator,
    )
    .await?
    {
        CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded),
        CoreExit::ResetWifi => Ok(CydWebCommand::ResetWifi),
        CoreExit::Reorientate(orientation) => Ok(CydWebCommand::Reorientate(orientation)),
    }
}
```

Everything in this function is a browser substitute for a capability the core
already requested:

| Core need | Browser construction |
| --- | --- |
| `Cyd + Button` | `CydWasm` and `ButtonWasm`, selected from `CydWebAppWasm` |
| Wi-Fi setup progression | `cyd_web_app_wasm.wifi_simulator` |
| `Dns` | `DnsSimulatorWasm` |
| Platform exit policy | `CydWebCommand` |

There is one deliberate simulation detail: `DnsSimulatorWasm` waits 12 ms before
returning a deterministic loopback address. Browsers cannot ask their operating
system for raw DNS timing, but they can honestly simulate the resolver
capability and give the shared core a measured, non-zero latency.

## Same exit, different policy

The core’s `Exit::Calibrate` still matters in WASM. The browser simply has a
different, truthful response:

```rust,no_run
CoreExit::Calibrate => Ok(CydWebCommand::CalibrationNotNeeded)
```

The framework releases transient input, restarts the app in the same
orientation, and queues one informational notice:

> Calibration is not needed in the browser.

This is better than pretending browser touch needs physical calibration, and
better than deleting the shared core control. A reader can see that the core
behavior is preserved while the platform policy differs for a reason.

`ResetWifi` is similarly browser-specific: the framework resets the simulated
Wi-Fi state and restarts at its connection phase. `Reorientate` persists the
orientation and reconstructs the canvas. Neither decision leaks into the core.

## A display-only app should say so

The container always owns the complete browser session, including the CYD
touch capability. A display-only launcher simply narrows that capability before
calling its core function, so the core still receives only what it requested.

Ballet needs a display and BOOT input, not touch. Its launcher says exactly
that:

```rust,no_run
const WEB_APP: CydWebAppConfig = CydWebAppConfig::new(
    "linkage-blaze/ballet",
    ORIENTATION,
    BACKGROUND,
    FOREGROUND,
    &TOP_FONT,
);

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, wasm_bindgen::JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}

async fn inner_main(
    mut cyd_web_app_wasm: CydWebAppWasm,
) -> Result<
    CydWebCommand,
    linkage_blaze_core::examples::ballet::Error<core::convert::Infallible>,
> {
    let mut display = cyd_web_app_wasm.cyd.display();
    match ballet(&mut display, &mut cyd_web_app_wasm.button).await {
        Ok(never) => match never {},
        Err(error) => Err(error),
    }
}
```

The unified supervisor constructs the complete `CydWebAppWasm` session. The
launcher narrows its display capability with `cyd_web_app_wasm.cyd.display()`;
it does not pass the broader CYD to display-only core code.

Clock and Skeleton Clock use this same display-only path. Their `inner_main`
functions narrow the container to `CydDisplayWasm` and `ButtonWasm`, then pass
the container's `ClockSyncWasm` and `WifiSimulatorWasm` to the core. Clock pages
also call `cyd_web_app_wasm.clock_sync.show()` so the shared shell exposes the
time control only for those applications.

For example, the important clock-specific part is:

```rust,no_run
async fn inner_main(mut cyd_web_app_wasm: CydWebAppWasm) -> Result<CydWebCommand, MainError> {
    cyd_web_app_wasm.clock_sync.show();
    let mut display = cyd_web_app_wasm.cyd.display();
    match clock(
        &mut display,
        &cyd_web_app_wasm.clock_sync,
        &mut cyd_web_app_wasm.button,
    )
    .await? {
        Exit::ResetWifi => Ok(CydWebCommand::ResetWifi),
    }
}
```

That distinction is not an optimization hidden in the framework. It is part of
the explanation. A core signature that asks for `CydDisplay` should result in a
launcher that narrows the framework-constructed `CydWasm` to `CydDisplay`.

## The framework is boring on purpose

The exported JavaScript surface stays small:

```rust,no_run
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<CydWebAppHandle, wasm_bindgen::JsValue> {
    start_cyd_web_app(canvas_id, WEB_APP, PAGE_INFO, inner_main)
}
```

The shared page shell mounts that handle, forwards canvas pointer events and the
BOOT control, and presents typed notices. It does not know the DNS Tester’s
exit strings, own a second restart loop, or emulate the app’s state machine in
JavaScript.

That is the payoff of putting lifecycle mechanics in the framework rather than
in each launcher. The launcher remains readable because it contains only
application-specific construction and policy.

## The pattern to copy

To put another CYD-related app on a web page:

1. Keep the core generic over the narrow capabilities it actually uses.
2. Always choose `start_cyd_web_app`; it supplies one complete
   `CydWebAppWasm` environment.
3. Narrow `CydWasm` to `CydDisplayWasm` with `.display()` when the core needs
   only display capability.
4. Select browser-only adapters—DNS, clock, or simulated Wi-Fi—from the
   container inside `inner_main`.
5. Translate the core’s returned `Exit` variants into adjacent,
   platform-specific `CydWebCommand` variants.

That is enough. No application-specific DOM wrapper class. No JavaScript
restart protocol. No imitation calibration sequence. No generic “web app”
trait that hides the real requirements.

The browser version is concise for the same reason the ESP version is concise:
each platform constructs the capabilities it can genuinely provide, then hands
the unchanged core the things it asked for.
