<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# One CYD Constructor, Two Platforms: Building the DNS Tester for ESP32 and WASM

## The prompt that generated this article

> One of our
> goals is to show how nice and sensible we can make the code. Let’s put that
> to the test by creating a pretend Medium article that shows the ESP
> construction, bit by bit, for the DNS Tester; the new general WASM-using code
> for the same constructor function on WASM; and then a brief overview of the
> core code they now share. Show the real code in parts and explain how the ESP
> and WASM constructors differ and why. Argue that both are concise, readable,
> and sensible, and that their differences are motivated. Put the article in
> `specs/`.

> Work backward from what the core application needs. Show those capabilities
> first, then construct each one at a time on ESP and WASM, including anything
> required by the constructors themselves. After that, handle each `Exit`
> variant individually and show how the platform applies the requested policy
> and restarts the core application.

*Pretend Medium article*

The most satisfying embedded code is not code that hides the hardware. It is
code that makes the hardware visible in the few places where it matters, while
keeping the application independent of the board.

The Device Envoy DNS Tester is a useful example. Its ESP32 entry point wires
real pins, Wi-Fi, flash, calibration, and reset behavior. Its WASM entry point
wires an HTML canvas, browser storage, browser input, animation frames, and a
deterministic substitute for Wi-Fi. After those jobs are complete, both
platforms run the same no-std DNS Tester loop.

That is the design in one sentence:

> Platform code constructs capabilities. Core code runs the device behavior.

## Start at the application boundary

The shared application does not ask whether it is running on an ESP32 or in a
browser. It asks for a CYD, a button, and a DNS service:

```rust,no_run
pub async fn dns_tester<CydDevice, ButtonDevice, DnsDevice>(
    cyd: &mut CydDevice,
    button: &mut ButtonDevice,
    dns: &mut DnsDevice,
) -> Result<Exit, Error<CydDevice::Error, DnsDevice::Error>>
where
    CydDevice: Cyd,
    ButtonDevice: Button,
    DnsDevice: Dns,
{
    // Shared state, layout, touch handling, rendering, and DNS accounting.
}
```

Notice what is not in this signature: flash, Wi-Fi configuration, a canvas,
SPI, or a reset function. Those are platform-owned construction and lifecycle
resources. They are used to prepare the three capabilities above and to act on
the `Exit` value afterward; they are deliberately not passed into the core
loop.

The loop owns the query, success, failure, and latency counters; the layout in
all four orientations; touch interpretation; the status display; and the
decision to request calibration, Wi-Fi reset, or reorientation. It returns an
`Exit` rather than clearing flash or rebooting the device:

```rust,no_run
pub enum Exit {
    Calibrate,
    ResetWifi,
    Reorientate(Orientation),
}
```

That return value is the seam between reusable application behavior and
platform policy.

## ESP32 construction: follow the actual dependency graph

The first core input is not merely a `Cyd`; it is a calibrated `Cyd`. That is
the right place to begin. The rest of the ESP entry point exists to make this
line possible:

```rust,no_run
let mut cyd = CydEsp::from_parts(display, touch);
```

At this point `display` is a real display and `touch` is calibrated. Neither
is an implementation detail that the core loop should have to reconstruct.

```text
calibrated CydEsp
├── display + uncalibrated touch
│   └── CydEspUncalibrated::new(..., display_orientation, ...)
│       ├── p from init_and_start!
│       └── display_orientation
│           ├── saved orientation from orientation flash
│           └── calibration availability from calibration flash
└── calibrated touch
    └── ensure_calibration(display, uncalibrated touch, calibration flash, BOOT)
```

### 1. The target: a calibrated `CydEsp`

`CydEsp::from_parts` is intentionally small because it is a type-level
handoff: it accepts only a display and calibrated touch. The calibration step
supplies that touch:

```rust,no_run
init_and_start!(p);
esp_println::logger::init_logger(log::LevelFilter::Info);

// ... construct display, calibrated touch, button, and DNS ...

let (touch, calibration_outcome) = ensure_calibration(
    &mut display,
    touch,
    &mut calibration_flash_block,
    &mut *button,
    Some("recalibrating"),
)
.await?;

if calibration_outcome.was_saved() {
    while button.is_pressed() {
        Timer::after(Duration::from_millis(10)).await;
    }
    device_envoy_esp::esp_hal::system::software_reset();
}

let mut cyd = CydEsp::from_parts(display, touch);
```

The reset after saving is part of the dependency story: a fresh boot now sees
the saved calibration and can construct the dashboard normally.

### 2. The input to calibration: an uncalibrated CYD

`ensure_calibration` needs an actual display and raw touch source, which come
from the uncalibrated constructor. Its visible dependency is
`display_orientation`:

```rust,no_run
static CYD_STATIC: CydStaticEsp<STATUS_PIXEL_COUNT> = CydEsp::new_static();
let CydEspUncalibrated { mut display, touch } = CydEspUncalibrated::new(
    &CYD_STATIC,
    p.SPI2,
    p.GPIO1, p.GPIO2, p.GPIO3,
    p.GPIO4, p.GPIO5, p.GPIO7, p.GPIO8,
    DEFAULT_DISPLAY_SPI_HZ,
    display_orientation,
    Rgb888::new(10, 10, 12), // near-black
    Rgb888::new(230, 230, 230), // near-white
    &DEFAULT_FONT,
    p.SPI3,
    p.GPIO9, p.GPIO10, p.GPIO11,
    p.GPIO12, p.GPIO13,
)?;
```

### 3. The uncalibrated CYD needs an orientation

Many applications use a fixed `Orientation` constant. DNS Tester is different:
the user can select a new orientation from the dashboard, so the startup
orientation must be loaded from persistent storage. Calibration is always
presented in landscape, which means calibration availability also participates
in computing `display_orientation`:

```rust,no_run
let orientation = orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);
let calibration_is_available = match calibration_flash_block.load::<CalibrationConfig>() {
    Ok(Some(_)) => true,
    Ok(None) | Err(_) => false,
};
let display_orientation =
    display_orientation_for_calibration(orientation, calibration_is_available);
```

### 4. Orientation and calibration state need board and flash resources

`init_and_start!` in the outer construction frame produces the board resources
named `p`. The real entry point then allocates all three persistent blocks
together from its single flash resource. The graph uses the orientation and
calibration blocks above, while the Wi-Fi block waits for the DNS capability:

```rust,no_run
let [
    wifi_auto_flash_block,
    mut calibration_flash_block,
    mut orientation_flash_block,
] = FlashBlockEsp::new_array::<3>(p.FLASH)?;

let button = DnsTesterButtonWatch::new(p.GPIO6, PressedTo::Ground, spawner).await?;
```

The BOOT button belongs here because it is the remaining dependency of
`ensure_calibration` and is later passed unchanged to the core loop.

### 5. Construct the DNS capability

The third core input is `Dns`. `WifiAutoEsp` consumes the Wi-Fi flash block,
real radio peripheral, and BOOT button to produce a connected network stack;
the small `DnsRuntime` closure adapts that stack to the core `Dns` contract:

```rust,no_run
let wifi_auto = WifiAutoEsp::new(
    p.WIFI,
    wifi_auto_flash_block,
    CAPTIVE_PORTAL_SSID,
    [],
    spawner,
)?;
let stack = wifi_auto.connect(&mut *button, /* Wi-Fi notices */).await?;

let mut dns = DnsRuntime::new(DNS_HOSTNAME, async || {
    let query_start = Instant::now();
    let dns_result = stack.dns_query(DNS_HOSTNAME, DnsQueryType::A).await;
    let latency_millis = query_start.elapsed().as_millis();
    Ok::<DnsResult, Infallible>(DnsResult {
        succeeded: matches!(dns_result, Ok(addresses) if !addresses.is_empty()),
        latency_millis,
    })
});
```

The exact GPIO assignments belong here, in the board entry point. The shared
application sees only the calibrated `CydEsp`, BOOT button, and `DnsRuntime`.

## WASM construction: the same dependency graph, different resources

The browser launcher is in `device-envoy-dns-tester-wasm`, while the reusable
CYD browser implementation lives in `device-envoy-core::wasm`. The target is
again a calibrated CYD, this time `CydWasm`:

```text
calibrated CydWasm
├── display + uncalibrated touch from CydSimulatorWasm
│   └── canvas + display_orientation + shared simulator style
└── calibrated touch from ensure_calibration
    └── browser calibration storage + browser BOOT source
```

### 1. The target: a calibrated `CydWasm`

After calibration, the launcher makes the same type-level handoff as ESP:

```rust,no_run
let mut device = CydWasm::from_parts(display, touch);
```

### 2. The uncalibrated simulator resources

The general, application-neutral constructor is
`CydSimulatorWasm::new_with_style`. It produces the display, raw touch, BOOT
source, and a browser control handle together:

```rust,no_run
let simulator = CydSimulatorWasm::new_with_style(
    self.canvas.clone(),
    orientation,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
)?;
let (device, mut button, simulator_control) = simulator.into_parts();
*self.simulator_control.borrow_mut() = Some(simulator_control);
let (mut display, uncalibrated_touch) = device.parts_uncalibrated();
```

The control handle stays with the browser shell so pointer and virtual BOOT
events continue to work while startup is awaiting calibration or animation
frames.

### 3. The simulator needs persistent orientation and calibration state

As on ESP, the DNS Tester loads orientation because the user can change it;
many other applications simply pass a fixed orientation constant. Browser
storage provides both the saved orientation and the answer to whether startup
must run calibration in landscape:

```rust,no_run
let saved_orientation = state
    .orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);
let calibration_config = state.calibration_flash_block.load::<CalibrationConfig>()?;
let orientation = display_orientation_for_calibration(
    saved_orientation,
    calibration_config.is_some(),
);
```

### 4. Calibrate, then rebuild if the orientation changes

The WASM launcher uses the same `ensure_calibration` operation as ESP:

```rust,no_run
let (touch, outcome) = ensure_calibration(
    &mut display,
    uncalibrated_touch,
    &mut state.calibration_flash_block,
    &mut button,
    Some("Touch calibrated"),
)
.await?;
```

If calibration was just saved, the browser rebuilds the simulator in the saved
dashboard orientation and applies the new calibration to its touch source. That
extra reconstruction is a browser-presentation concern; it keeps canvas size,
orientation, input control, and calibrated touch in agreement.

### 5. Construct the deterministic DNS capability

The browser has no equivalent of an ESP32 Wi-Fi stack, so it simulates the
connection phase and supplies a deterministic `DnsRuntime`. BOOT can interrupt
the simulated connection just as it can interrupt ESP Wi-Fi setup:

```rust,no_run
let wifi_outcome = simulate_wifi_connect(&mut button, async |event| {
    self.request_notice(match event {
        WifiConnectEvent::CaptivePortalReady => SimulatorNoticeRequest::wifi_setup(),
        WifiConnectEvent::Connecting { .. } => SimulatorNoticeRequest::wifi_connecting(),
        WifiConnectEvent::ConnectionFailed => SimulatorNoticeRequest::wifi_unavailable(),
    });
    Ok::<(), JsValue>(())
})
.await?;

let mut dns = DnsRuntime::new(hostname, async || {
    Ok::<DnsResult, Infallible>(DnsResult {
        succeeded: true,
        latency_millis: 12,
    })
});
```

`ConnectionFailed` remains a hook for explicit failure injection; the normal
simulation does not invent failures. The browser now has the same three core
inputs as ESP: calibrated CYD, BOOT button, and DNS service.

### 6. Spawn the shared loop and report its exit

The browser task must not block JavaScript, so it runs the same core loop in
`spawn_local`:

```rust,no_run
let mut device = CydWasm::from_parts(display, touch);
let exit = self.exit.clone();
let failed = self.failed.clone();

wasm_bindgen_futures::spawn_local(async move {
    match dns_tester(&mut device, &mut button, &mut dns).await {
        Ok(exit_value) => exit.set(Some(exit_value)),
        Err(CoreError::Display(CoreUiError::Text(_))) => failed.set(true),
        Err(CoreError::Display(CoreUiError::Display(error))) => match error {},
        Err(CoreError::Touch(error)) => match error {},
        Err(CoreError::Dns(error)) => match error {},
    }
});
```

The page shell later calls `take_exit` and `take_notice`. That is the browser
version of the ESP entry point’s final `match exit`: the shell performs the
restart or orientation update appropriate to a web page.

## Notices and failures cross a typed boundary

The Rust launcher submits `SimulatorNoticeRequest` values containing a stable
identifier and severity. Recoverable Wi-Fi notices continue the application;
the fatal `runtime-error` notice marks the browser task as failed. The shared
JavaScript shell owns placement, timeout, replacement, and accessibility roles.
JavaScript does not infer severity from message text.

After the shared loop is spawned with `spawn_local`, the launcher exposes typed
state to the shell through `take_exit` and `take_notice`. A normal exit can
request recalibration, Wi-Fi reset, or orientation persistence. A display text
failure becomes `runtime error`; the infallible display, touch, and DNS errors
are handled exhaustively.

## The small shared core

The platform setup is substantial only because it has real platform work to
do. The application loop that follows is deliberately small and boring. It
reads the button, reads calibrated touch, renders the current status, and
performs a lookup only when the shared UI reports a `StartDns` action:

```rust,no_run
loop {
    yield_now().await;

    if button.is_pressed() {
        return Ok(Exit::Calibrate);
    }

    ui.begin(touch.read()?, orientation);
    ui.status(layout.status, status, status.is_good()).await?;

    match ui.touch(layout) {
        TouchAction::StartDns => {
            let result = dns.lookup().await?;
            queries = queries.saturating_add(1);
            last_latency_millis = Some(result.latency_millis);
            // Update the shared success/failure state and render it next frame.
        }
        TouchAction::Control(Control::Calibration) => {
            return Ok(Exit::Calibrate);
        }
        TouchAction::Control(Control::Wifi) => return Ok(Exit::ResetWifi),
        TouchAction::Control(Control::Orientation) => {
            return Ok(Exit::Reorientate(orientation.next()));
        }
        TouchAction::None => {}
    }
}
```

The real function also renders the counters, latency, bitmap, and status
details; the important point here is ownership. No branch in this loop knows
whether `touch` came from an XPT2046 controller or browser pointer events, or
whether `dns.lookup()` uses a socket or a deterministic closure.

## Three exits, three platform policies

The return value from the core loop is not an abstract promise to “restart
somehow.” Each variant has a precise meaning. Looking at the exits one at a
time makes the boundary between core behavior and platform behavior especially
clear.

### `Calibrate`: clear calibration, then start fresh

The core loop returns `Calibrate` when the physical BOOT button is pressed or
when the on-screen CAL control is touched. The ESP implementation clears only
the calibration block and then resets:

```rust,no_run
match exit {
    Exit::Calibrate => {
        calibration_flash_block.clear()?;
    }
    // ...the other exits...
}
device_envoy_esp::esp_hal::system::software_reset();
```

On the next hardware boot, startup sees that calibration is absent, constructs
the CYD in the calibration orientation, and runs `ensure_calibration` again.
The saved Wi-Fi and dashboard orientation are left alone.

The browser performs the same policy through its page shell. It clears only
calibration storage, releases transient BOOT input, switches the canvas to
landscape, and calls `reboot`, which calls `start` again:

```javascript
if (result === "recalibrate") {
  tester.clear_calibration();
  tester.boot_up();
  tester.prepare_calibration_landscape();
  syncPresentation();
  await tester.reboot();
  syncPresentation();
}
```

This is a browser restart rather than a machine reset, but the application
observes the same lifecycle: construct, calibrate, and re-enter the core loop.

### `ResetWifi`: preserve the device, restart the connection phase

The core loop returns `ResetWifi` from the Wi-Fi control region. ESP delegates
the policy to `WifiAutoEsp`, which clears its connection state back to the
captive portal before the normal reset path runs:

```rust,no_run
match exit {
    Exit::ResetWifi => {
        wifi_auto.reset_to_captive_portal()?;
    }
    // ...the other exits...
}
device_envoy_esp::esp_hal::system::software_reset();
```

The next boot reconstructs Wi-Fi, waits for link and DHCP, and only then
re-enters `dns_tester`. The DNS counters therefore belong to the new core-loop
run, while persistent calibration and orientation remain available.

In the browser, the shell releases the virtual BOOT button and invokes the
same `reboot` entry point. `start` reconstructs the simulator, runs the
deterministic captive-portal and connecting phases, and then spawns the core
loop again:

```javascript
if (result === "wifi") {
  tester.boot_up();
  await tester.reboot();
  syncPresentation();
}
```

The browser does not pretend to reset a Wi-Fi chip. It restarts the simulated
connection service that the application actually depends on.

### `Reorientate`: persist the next display orientation

The core loop returns `Reorientate(orientation.next())` when the ROT control is
used. On ESP, the entry point saves the new orientation and resets:

```rust,no_run
match exit {
    Exit::Reorientate(next_orientation) => {
        orientation_flash_block.save(&next_orientation)?;
    }
    // ...the other exits...
}
device_envoy_esp::esp_hal::system::software_reset();
```

The next construction pass loads that value before creating the display, so
the first frame is already in the requested orientation.

On WASM, `take_exit` performs the persistence and updates the intrinsic canvas
dimensions before returning the string `"orientation"` to JavaScript. The
shell synchronizes the presentation and then calls `reboot`:

```javascript
if (result === "orientation") {
  syncPresentation();
  await tester.reboot();
  syncPresentation();
}
```

This ordering matters. The browser must resize and relayout the shell before
the restarted splash is drawn, while the ESP can let its next hardware boot
perform that work naturally.

### The restart loop is still platform-specific, but the lifecycle is shared

The ESP’s restart loop is supplied by the microcontroller itself: return from
the core loop, apply one policy, reset, and call `inner_main` again after boot.
The WASM shell expresses the same lifecycle explicitly:

```javascript
while (true) {
  await nextFrame();
  const result = tester.take_exit();

  if (result === "recalibrate" || result === "wifi" || result === "orientation") {
    // Apply the variant-specific policy, then call tester.reboot().
    continue;
  }
  if (result === "runtime error") {
    return;
  }
}
```

The shared core requests a transition. Each platform performs the transition
with the resources and restart mechanism it owns, then constructs the same core
application again.

## What the two constructors share

| Concern | ESP32 | WASM |
| --- | --- | --- |
| Display | SPI pins, ILI9341 driver, static framebuffer | HTML canvas and `CydSimulatorWasm` |
| Touch | XPT2046 controller plus calibration | Browser pointer events mapped by simulator control |
| BOOT | GPIO button | Browser BOOT control |
| Persistence | ESP flash blocks | `localStorage`-backed `FlashBlockWasm` |
| Wi-Fi | `WifiAutoEsp`, DHCP, live DNS | `simulate_wifi_connect`, deterministic DNS result |
| Restart | Software reset | Typed exit observed by the page shell, then `start`/`reboot` |
| Scheduling | Embassy timers | `spawn_local` and animation-frame futures |

The shared pieces are more important than the table suggests: the `Cyd` and
`Dns` abstractions, calibrated touch events, orientation model, calibration
flow, splash renderer, DNS accounting, UI layout, and control exits are the
same. Only capabilities that truly differ are adapted.

## Why the differences are sensible

Both constructors are concise because they have one job: turn platform
resources into the capabilities expected by the shared loop. They are readable
because the construction order follows the device’s actual startup story:
restore state, construct hardware or browser resources, calibrate, start the
platform service, and run the application.

They are not identical, and that is a feature. ESP needs GPIOs, SPI, flash,
DHCP, and a software reset. WASM needs a canvas, `localStorage`, browser input,
animation frames, typed notices, and a shell-visible restart. Making those
differences explicit is more honest—and easier to review—than forcing both
platforms through a misleading universal constructor.

## The architectural test

Read `dns_tester` without knowing its platform. It should still make complete
sense. Read the ESP entry point and the pins, flash, Wi-Fi, calibration, and
reset policy should be obvious. Read `DnsTesterWeb::start` and the canvas,
browser storage, simulator control, typed notices, animation scheduling, and
deterministic DNS should be equally obvious.

That is the standard worth aiming for: constructors explicit enough to trust,
shared code broad enough to reuse, and differences that correspond to actual
platform capabilities rather than accidental duplication.
