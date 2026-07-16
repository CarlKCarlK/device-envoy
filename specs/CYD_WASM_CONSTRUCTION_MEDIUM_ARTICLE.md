<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

## The prompt that generated this article

> One of our goals is to show how nice and sensible we can make the code. Let’s
> put that to the test by creating a pretend Medium article that shows the ESP
> construction, bit by bit, for the DNS Tester; the new general WASM-using code
> for the same constructor function on WASM; and then a brief overview of the
> core code they now share. Show the real code in parts and explain how the ESP
> and WASM constructors differ and why. Argue that both are concise, readable,
> and sensible, and that their differences are motivated. Put the article in
> `specs/`.
>
> Work backward from what the core application needs. Show those capabilities
> first, then construct each one at a time on ESP and WASM, including anything
> required by the constructors themselves. After that, handle each `Exit`
> variant individually and show how the platform applies the requested policy
> and restarts the core application.

# One CYD Constructor, Two Platforms

The Device Envoy DNS Tester has one shared application and several platform
front ends. The application owns the dashboard, touch interpretation, DNS
accounting, and rendering. A platform owns the resources needed to make those
things possible.

That division is easiest to see at the application boundary.

## Begin with the shared function

The hardware path calls one shared function:

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
    // Render the dashboard, read touch and button input,
    // and perform DNS queries.
}
```

The function does not know about SPI pins, flash blocks, Wi-Fi credentials, an
HTML canvas, or a reset mechanism. It returns an exit request instead:

```rust,no_run
pub enum Exit {
    Calibrate,
    ResetWifi,
    Reorientate(Orientation),
}
```

The meaning of an exit is shared. Its implementation is not. ESP can clear a
calibration block, reset Wi-Fi, or reboot. WASM can explain that calibration or
Wi-Fi reset requires physical hardware, or persist a new orientation and
restart the browser wrapper. Memory tests can use the exit simply as a
deterministic stopping point.

## ESP construction

The ESP entry point constructs the CYD and the button before entering the
application. `CydEsp::new` receives the display and touch resources, the
orientation, drawing style, calibration storage, and calibration button. The
constructor owns the calibration details and returns a ready-to-use CYD.

The application never receives an intermediate construction state:

```rust,no_run
let [wifi_auto_flash_block, mut calibration_flash_block, mut orientation_flash_block] =
    FlashBlockEsp::new_array::<3>(p.FLASH)?;
let orientation = orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);

static CYD_STATIC: CydStaticEsp<FRAME_PIXEL_COUNT> = CydEsp::new_static();
let button = ButtonWatch::new(button_pin, PressedTo::Ground, spawner).await?;
let mut cyd = CydEsp::new(
    &CYD_STATIC,
    display_spi,
    display_pins,
    DEFAULT_DISPLAY_SPI_HZ,
    orientation,
    Rgb888::new(10, 10, 12),
    Rgb888::new(230, 230, 230),
    &DEFAULT_FONT,
    touch_spi,
    touch_pins,
    &mut calibration_flash_block,
    &mut *button,
)
.await?;
```

Boards with one SPI peripheral use the same boundary through
`CydEspOneSpi::new`. The display and touch devices share a bus internally, but
the caller still receives one ready CYD and passes it to the same application.

After construction, ESP shows the shared splash, connects Wi-Fi, and calls the
shared loop:

```rust,no_run
dns_tester::splash(&mut cyd).await?;

let stack = wifi_auto
    .connect(&mut *button, async |event| {
        dns_tester::wifi_status(&mut cyd, event).await?;
        Ok(())
    })
    .await?;

let mut dns = DnsWithStack::new(*stack);
match dns_tester::run(&mut cyd, &mut *button, &mut dns).await? {
    Exit::Calibrate => calibration_flash_block.clear()?,
    Exit::ResetWifi => wifi_auto.reset_to_captive_portal()?,
    Exit::Reorientate(next_orientation) => {
        orientation_flash_block.save(&next_orientation)?;
    }
}
software_reset();
```

The ESP entry point is therefore a resource-construction and policy layer. It
does not duplicate dashboard behavior.

## WASM construction

WASM has different capabilities. It has a canvas instead of an SPI display,
browser pointer events instead of a physical touch controller, browser storage
instead of flash, and deterministic DNS instead of a Wi-Fi stack.

It does not need a separate application loop. It constructs a simulator and
calls the same `dns_tester::run` function.

### Restore orientation and construct the simulator

The current WASM state contains one persistent value: orientation.

```rust,no_run
let orientation = state
    .orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);
self.orientation.set(orientation);
state.orientation = orientation;
self.canvas.set_width(orientation.width());
self.canvas.set_height(orientation.height());

let simulator = CydSimulatorWasm::new_with_style(
    self.canvas.clone(),
    orientation,
    BACKGROUND,
    FOREGROUND,
    &FONT_6X10,
)?;
let (mut device, mut button, simulator_control) = simulator.into_parts();
*self.simulator_control.borrow_mut() = Some(simulator_control);
```

The simulator supplies a CYD, a browser-controlled button, and the input
adapter. The browser shell forwards pointer and boot-button events to that
adapter. There is no separate WASM calibration construction phase.

### Render and start asynchronously

The browser renders its startup notice directly, then yields animation frames
before starting the application task:

```rust,no_run
let mut display = device.display();
render_notice(&mut display, orientation, UiNotice::Splash).await?;
for _ in 0..60 {
    next_animation_frame().await;
}

let mut dns = MockDns;
let exit = self.exit.clone();
let failed = self.failed.clone();
wasm_bindgen_futures::spawn_local(async move {
    match dns_tester::run(&mut device, &mut button, &mut dns).await {
        Ok(exit_value) => exit.set(Some(exit_value)),
        Err(_) => failed.set(true),
    }
});
```

`MockDns` returns a deterministic loopback address. The browser never claims
to have connected Wi-Fi; it supplies only the `Dns` capability the shared loop
requires.

## The same controls, different policies

The dashboard contains calibration, Wi-Fi, and orientation controls for the
hardware application. WASM calls the same `run`, so those controls produce the
same `Exit` variants. The difference is what happens afterward.

For orientation, WASM saves the next orientation, resizes the canvas, and
returns an orientation result to JavaScript. The page synchronizes its layout
and restarts the wrapper.

For calibration and Wi-Fi reset, WASM returns stable unsupported results. The
page displays an explanation and restarts the dashboard:

```javascript
if (result === "calibration unavailable") {
  await rebootAndSyncStage(syncPresentation);
  showNotice({
    severity: "info",
    message: "Touch calibration is available on physical CYD hardware only.",
  });
}

if (result === "wifi reset unavailable") {
  await rebootAndSyncStage(syncPresentation);
  showNotice({
    severity: "info",
    message: "Wi-Fi reset is available on physical CYD hardware only.",
  });
}
```

This is preferable to hiding or disabling the shared controls. The browser
exercises the same application behavior while its platform boundary explains
which hardware policy cannot be applied.

## Memory uses the same function too

The memory test double supplies a `ButtonMemory`, a `CydMemory`, and a test DNS
implementation. It calls `run`, scripts a touch on the desired control, and
asserts the resulting exit:

```rust,no_run
let mut button = cyd_memory.button_memory();
let mut dns = SuccessfulDns;
let exit = block_on(dns_tester::run(
    &mut cyd_memory,
    &mut button,
    &mut dns,
))?;
assert_eq!(exit, Exit::Reorientate(Orientation::Portrait));
```

Memory does not need to emulate a physical calibration procedure or a Wi-Fi
reset. Its test harness can treat any exit as the end of the scripted run and
reset or discard the in-memory device before the next test. The important
property is that the production entry point remains the same everywhere.

## What is shared

| Concern | ESP32 | WASM | Memory |
| --- | --- | --- | --- |
| CYD | `CydEsp` or `CydEspOneSpi` | `CydSimulatorWasm` | `CydMemory` |
| Button | GPIO-backed `ButtonWatch` | simulator-controlled button | `ButtonMemory` |
| DNS | connected `DnsWithStack` | deterministic `MockDns` | test DNS double |
| Main loop | `dns_tester::run` | `dns_tester::run` | `dns_tester::run` |
| Exit policy | clear/reset/save, then reboot | explain or save, then restart | terminate/reset test state |

The shared loop owns layout, rendering, touch interpretation, DNS query
accounting, latency display, and exit generation. Platform code owns resources
and policy. That is the useful boundary: one application, one public run path,
and honest differences where a platform cannot perform an operation.

## The architectural test

Read the core DNS tester without knowing which platform will call it. The
function should still make sense. Read the ESP entry point and the CYD
constructor, calibration storage, Wi-Fi connection, and reset policy should be
visible. Read `DnsTesterWeb::start` and the canvas, orientation storage,
simulator input, animation scheduling, deterministic DNS, and unsupported-exit
notices should be visible.

The code is concise not because the platforms are forced to look alike, but
because each platform constructs exactly the capabilities it can provide and
the shared application consumes exactly those capabilities.
