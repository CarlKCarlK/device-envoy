<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# One DNS Tester, Two Worlds: Why the ESP and WASM Constructors Stay Small

*Pretend Medium article*

The most satisfying embedded code is not code that hides the hardware. It is code
that makes the hardware visible in the few places where it matters, while keeping
the application itself independent of the board.

The Device Envoy DNS Tester is a useful example. The ESP32 constructor and the
browser/WASM constructor look different because they have different jobs. The ESP
constructor wires real pins, Wi-Fi, flash, and reset behavior. The WASM constructor
wires a canvas, browser storage, and deterministic substitutes. After that setup,
both constructors call the same no-std DNS Tester loop.

That is the whole design in one sentence:

> Platform code constructs capabilities. Core code runs the device behavior.

## Start with the application boundary

The shared application does not ask whether it is running on an ESP32 or in a
browser. It asks for three capabilities:

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

This signature is intentionally ordinary. `Cyd` supplies display and touch
parts. `Button` supplies the physical BOOT/calibration input. `Dns` supplies a
hostname and one asynchronous lookup operation.

The core loop owns the things that should be identical everywhere:

- the query, success, failure, and latency counters;
- the screen layout in each orientation;
- touch interpretation and control regions;
- the status display;
- the decision to request calibration, Wi-Fi reset, or reorientation.

It returns an `Exit` instead of rebooting or changing flash itself:

```rust,no_run
pub enum Exit {
    Calibrate,
    ResetWifi,
    Reorientate(Orientation),
}
```

That return value is the seam between reusable application behavior and
platform policy.

## The ESP constructor, one responsibility at a time

The ESP32 entry point is longer than the WASM entry point because real hardware
has more physical facts to establish. The length is not accidental ceremony;
each block answers one concrete question.

### 1. Start the board and persistent services

```rust,no_run
async fn inner_main(spawner: Spawner) -> Result<Infallible> {
    init_and_start!(p);
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let [
        wifi_auto_flash_block,
        mut calibration_flash_block,
        mut orientation_flash_block,
    ] = FlashBlockEsp::new_array::<3>(p.FLASH)?;
```

The board starts first. Then the constructor obtains three persistent namespaces:
Wi-Fi setup, touch calibration, and display orientation. These are real flash
blocks, so the constructor can preserve the user’s device state across power
cycles.

### 2. Restore the display state

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
    let mut button = ButtonEsp::new(p.GPIO0, PressedTo::Ground);
```

There are two orientations here: the saved dashboard orientation and the
temporary orientation used while calibration is running. This is a hardware
detail that belongs in the constructor because the constructor owns startup and
restart policy.

### 3. Construct the actual CYD

```rust,no_run
    static CYD_STATIC: CydStaticEsp<STATUS_PIXEL_COUNT> = CydEsp::new_static();
    let CydEspUncalibrated { mut display, touch } = CydEspUncalibrated::new(
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
        display_orientation,
        Rgb888::new(10, 10, 12), // near-black
        Rgb888::new(230, 230, 230), // near-white
        &DEFAULT_FONT,
        p.SPI3,
        p.GPIO25,
        p.GPIO32,
        p.GPIO39,
        p.GPIO33,
        p.GPIO36,
    )?;
```

This is the part that should look unmistakably like embedded code. SPI buses,
GPIO assignments, display colors, font, and a static memory budget are explicit.
Nothing generic is pretending that GPIO14 is a browser canvas.

The static buffer is also a deliberate design choice. The DNS Tester runs beside
Wi-Fi, so a full-screen framebuffer may be an unacceptable memory tradeoff. The
constructor selects a small buffer budget suitable for calibration text and the
single-line status display.

### 4. Calibrate before handing the device to the app

```rust,no_run
    let (touch, calibration_outcome) = ensure_calibration(
        &mut display,
        touch,
        &mut calibration_flash_block,
        &mut button,
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

Calibration is not in the generic DNS loop. It is a startup concern, it uses
persistent flash, and on this platform it may require a real software reset.
Keeping it here makes the shared loop easier to read and keeps ESP-specific
restart behavior out of application code.

### 5. Adapt real Wi-Fi to the tiny `Dns` contract

```rust,no_run
    let wifi_auto = WifiAutoEsp::new(
        p.WIFI,
        wifi_auto_flash_block,
        CAPTIVE_PORTAL_SSID,
        [],
        spawner,
    )?;
    let stack = wifi_auto.connect(&mut button, /* Wi-Fi notices */).await?;

    while !stack.is_link_up() || stack.config_v4().is_none() {
        Timer::after(Duration::from_millis(200)).await;
    }

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

The Wi-Fi implementation is necessarily platform-specific. The application does
not need to know about CYW43, sockets, DHCP, or DNS packet types. It receives the
small `DnsResult` it needs.

### 6. Run the shared application and interpret its request

```rust,no_run
    let exit = dns_tester(&mut cyd, &mut button, &mut dns).await?;
    match exit {
        Exit::Calibrate => calibration_flash_block.clear()?,
        Exit::ResetWifi => wifi_auto.reset_to_captive_portal()?,
        Exit::Reorientate(next_orientation) => {
            orientation_flash_block.save(&next_orientation)?;
        }
    }
    device_envoy_esp::esp_hal::system::software_reset();
}
```

The constructor handles policy after the core loop returns. That is concise and
readable precisely because the generic loop does not need to contain any of it.

## The WASM constructor: the same shape, different materials

The browser constructor has no pins and no physical reboot. It still follows the
same rhythm: restore state, construct a device, perform startup, provide a DNS
adapter, and launch the shared loop.

### 1. Construct browser substitutes

```rust,no_run
#[wasm_bindgen(constructor)]
pub fn new(canvas: HtmlCanvasElement) -> Result<DnsTesterWeb, JsValue> {
    Ok(Self {
        canvas,
        exit: Rc::new(Cell::new(None)),
        failed: Rc::new(Cell::new(false)),
        state: RefCell::new(DnsTesterState {
            wifi_flash_block: FlashBlockWasm::new("device-envoy/dns-tester/wifi")?,
            calibration_flash_block: FlashBlockWasm::new(
                "device-envoy/dns-tester/calibration",
            )?,
            orientation_flash_block: FlashBlockWasm::new(
                "device-envoy/dns-tester/orientation",
            )?,
            simulator_control: None,
            orientation: Orientation::Landscape,
            hostname: DNS_HOSTNAME,
        }),
    })
}
```

The browser has substitutes for the same durable concepts. `FlashBlockWasm`
uses browser storage. `Rc<Cell<_>>` and `RefCell<_>` hold the small amount of
state needed to supervise an asynchronous browser task. None of this is a fake
ESP constructor; it is a browser constructor with browser-appropriate services.

### 2. Construct the shared WASM CYD

```rust,no_run
let simulator = CydSimulatorWasm::new_with_style(
    self.canvas.clone(),
    orientation,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
)?;
let (device, mut button, simulator_control) = simulator.into_parts();
state.simulator_control = Some(simulator_control);
let (mut display, uncalibrated_touch) = device.parts_uncalibrated();
```

The canvas is the display. The shared WASM simulator provides touch, BOOT, flash,
orientation, and canvas presentation behavior. The constructor still explicitly
chooses the orientation, palette, and font, just as the hardware constructor
explicitly chooses pins and memory.

### 3. Reuse calibration and startup

```rust,no_run
let (touch, outcome) = ensure_calibration(
    &mut display,
    uncalibrated_touch,
    &mut state.calibration_flash_block,
    &mut button,
    Some("Touch calibrated"),
)
.await?;

dns_tester_splash(&mut display, state.orientation).await?;
for _ in 0..60 {
    next_animation_frame().await;
}
```

This is an important test of the abstraction. The same calibration and splash
functions operate on a browser-backed `CydDisplay`. The browser-specific delay is
explicit because a browser needs to yield animation frames; it is not smuggled
into the core DNS logic.

### 4. Replace live DNS with a deterministic browser service

```rust,no_run
let mut dns = DnsRuntime::new(hostname, async || {
    Ok::<DnsResult, Infallible>(DnsResult {
        succeeded: true,
        latency_millis: 12,
    })
});
```

Browsers cannot provide the same arbitrary DNS operation as the embedded Wi-Fi
stack. The WASM constructor therefore supplies a deterministic result. This is
not pretending that a browser performed a network measurement; it makes the
application’s rendering and interaction testable while accurately documenting
the platform limitation.

### 5. Launch the same loop without blocking the browser

```rust,no_run
let mut device = CydWasm::from_parts(display, touch);
let exit = self.exit.clone();
let mut dns = dns;

wasm_bindgen_futures::spawn_local(async move {
    match dns_tester(&mut device, &mut button, &mut dns).await {
        Ok(exit_value) => exit.set(Some(exit_value)),
        Err(_) => failed.set(true),
    }
});
```

The browser task is spawned rather than awaited by the exported start method.
The shell remains responsive, animation-frame futures can resolve, and a typed
core exit can be observed by the page wrapper.

## What both constructors share

The two entry points share more than a function name. They share the device
semantics underneath the constructor:

```rust,no_run
pub trait Dns {
    type Error;

    fn hostname(&self) -> &'static str;

    fn lookup(&mut self) -> impl Future<Output = Result<DnsResult, Self::Error>>;
}
```

They share the `Cyd` device abstraction, `CydDisplay`, calibrated touch events,
the `Button` contract, the `DnsRuntime` adapter, the orientation model, the
calibration flow, the splash renderer, the UI layout, and the complete DNS
accounting loop.

They also share the same error boundaries. A display error remains a display
error. A touch error remains a touch error. A DNS error remains a DNS error. The
hardware constructor maps those errors into its board error type; the WASM
constructor reports them through `JsValue` or its browser task state.

## Why the differences are justified

The constructors are not identical because identical code would be suspicious.

| Concern | ESP32 constructor | WASM constructor | Why the difference is correct |
| --- | --- | --- | --- |
| Display | SPI pins, static framebuffer, hardware driver | HTML canvas and `CydSimulatorWasm` | Different physical surfaces require different adapters. |
| Touch | XPT2046 controller and calibration samples | Pointer events forwarded by the browser shell | The input source is different; calibrated core touch events are the same. |
| BOOT | GPIO button | Browser BOOT control | Same `Button` capability, different source. |
| Flash | MCU flash blocks | Browser storage-backed blocks | Both preserve calibration and orientation, but use native persistence mechanisms. |
| Wi-Fi | CYW43 setup, DHCP, captive portal, live DNS | No Wi-Fi setup; deterministic DNS result | The browser cannot provide the embedded service honestly. |
| Restart | Software reset after a control request | Page wrapper observes a typed exit and restarts the task | Hardware reset and browser restart have different meanings. |
| Scheduling | Embassy timers and hardware task scheduling | Animation frames and `spawn_local` | Each platform yields through its native event loop. |

The differences are therefore motivated by capabilities, not by application
behavior. The constructors are concise because each one contains only the
platform facts that the other platform cannot provide.

## A useful test for good embedded architecture

Read the shared `dns_tester` function without knowing which platform called it.
It should still make complete sense.

Then read the ESP constructor. It should be obvious where pins, persistent
storage, calibration, Wi-Fi, and reboot policy enter.

Then read the WASM constructor. It should be obvious where canvas, browser
storage, deterministic DNS, and animation scheduling enter.

That is the standard worth aiming for: both constructors are short enough to
review, explicit enough to trust, and different only where the platform really
is different. The code is not clever. The abstraction is doing its job.
