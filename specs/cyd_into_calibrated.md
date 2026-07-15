# Spec: Move CYD Calibration onto the Uncalibrated Bundle

## Status

Proposed implementation plan for the `dev2026may` branch.

## Goal

Change the ESP32 two-SPI DNS tester startup so that CYD setup has two explicit steps:

1. Construct a complete uncalibrated CYD bundle.
2. Consume that bundle and return a complete calibrated CYD bundle.

The resulting `inner_main` should not manually split display and touch, call the low-level calibration driver, inspect a calibration outcome, wait for a button release, reconstruct the CYD, or reset after successful calibration.

The intended call site is:

```rust
let uncalibrated_cyd = CydEspUncalibrated::new(/* visible two-SPI wiring */)?;

let mut cyd = uncalibrated_cyd
    .into_calibrated(
        &mut calibration_flash_block,
        &mut *button,
    )
    .await?;
```

`into_calibrated` must return only:

- `Err(error)` for an unrecoverable error; or
- `Ok(calibrated_cyd)` for a CYD that is ready for application use.

It must not return `Option`, a calibration outcome, or a restart-required variant.

## Scope

Implement the complete behavior for:

- `CydEspUncalibrated`, using two independent SPI peripherals; and
- the generated ESP DNS tester `inner_main` that currently performs calibration manually.

Add the new trait method to other uncalibrated CYD bundle implementations, but leave those implementations as explicit temporary stubs:

```rust
todo!("todo0000 fix this up")
```

Do not migrate the other examples yet. Add `todo0000` comments to their source templates so they can be considered individually later.

## Non-goals

This change does not:

- redesign Wi-Fi setup;
- redesign DNS querying;
- clean up the DNS tester's unrelated nested error mappings;
- migrate every ESP, RP, WASM, or memory example;
- implement the final one-SPI, RP, WASM, or memory calibration transitions;
- make one-SPI CYDs implement `CydParts`; or
- introduce a generic board-construction function that hides hardware wiring.

## Public abstraction

Add a bundle-level trait named `CydUncalibrated` in `device-envoy-core::cyd`.

This trait is distinct from `CydTouchUncalibrated`:

- `CydTouchUncalibrated` represents only a raw touch device.
- `CydUncalibrated` represents a complete display-plus-raw-touch CYD bundle.

A signature in this general shape is desired:

```rust
pub trait CydUncalibrated: Sized {
    type Calibrated: Cyd;
    type Error;

    async fn into_calibrated<F, B>(
        self,
        calibration_flash_block: &mut F,
        recalibration_button: &mut B,
    ) -> Result<Self::Calibrated, Self::Error>
    where
        F: FlashBlock,
        B: Button,
        Self::Error: From<F::Error>;
}
```

The exact spelling may use `fn -> impl Future` instead of `async fn` if that better matches crate policy. The semantic contract is mandatory:

- The method consumes `self`.
- Success always returns a complete calibrated CYD.
- Recoverable calibration problems remain inside the method.
- Only unrecoverable device or persistence failures return `Err`.
- The method owns calibration-specific user messages and button handling.
- The caller does not inspect how calibration was obtained.
- The caller does not decide whether calibration should retry.
- The caller does not reconstruct the CYD from owned parts.
- The caller does not reset merely because a new calibration was saved.

The calibrated associated type should preserve the concrete bundle relationship. For the initial implementation:

```rust
impl CydUncalibrated for CydEspUncalibrated {
    type Calibrated = CydEsp;
    type Error = device_envoy_esp::Error;

    // Real implementation.
}
```

## Calibration behavior

### Recoverable calibration failures

Bad taps and other ordinary calibration failures must be handled by restarting or continuing the calibration flow inside `into_calibrated`.

Examples include:

- degenerate calibration geometry;
- residual error above the accepted limit;
- missing the verification target;
- verification timeout; and
- any other condition for which another user attempt is reasonable.

These conditions must not return `None`, `RestartRequired`, or `Err`.

### Unrecoverable failures

Return `Err` only when startup cannot reasonably continue, such as:

- display communication failure;
- raw-touch communication failure;
- failure to persist a validated calibration; or
- another platform error that prevents completing the operation.

The DNS tester already calls the method with `.await?`, so no separate `try_` prefix or caller-side branch is needed.

### Successful calibration

Whether the configuration was loaded from flash or freshly collected and saved, `into_calibrated` must return the same result shape:

```rust
Ok(CydEsp)
```

The method should display its own standard confirmation text when a fresh calibration is saved. Remove the caller-supplied `Option<&str>` message parameter.

The method should also own any required release/debounce behavior for the calibration button. No calibration-specific button polling belongs in `inner_main`.

## Final orientation

The returned CYD must already be in the requested application orientation.

Today, the DNS tester:

1. probes the calibration flash block;
2. constructs the display in landscape when calibration is absent;
3. saves a fresh calibration;
4. resets; and
5. reconstructs the display in the saved application orientation.

That policy must move below `inner_main`.

`CydEspUncalibrated::new` should continue to receive the requested final orientation. The uncalibrated bundle may store that orientation separately from the temporary orientation used during calibration.

Before `into_calibrated` returns, it must put the display and calibrated touch bundle into the requested final orientation. If the display driver lacks an in-process orientation change operation, add one or otherwise rework the uncalibrated bundle so the final CYD can be produced without a software reset.

Do not preserve the current reset merely as an implementation shortcut. If producing the final orientation without reset proves impossible, stop and revisit this API design rather than adding `Option<CydEsp>` or a restart outcome.

## Supporting calibration-driver refactor

The current low-level calibration helper consumes an uncalibrated touch object and returns a calibrated touch plus `EnsureCalibrationOutcome`.

For bundle-level conversion, prefer a lower-level helper that borrows the raw touch and returns only a valid configuration:

```rust
pub async fn ensure_calibration_config<D, T, F, B>(
    display: &mut D,
    touch: &mut T,
    calibration_flash_block: &mut F,
    recalibration_button: &mut B,
) -> Result<CalibrationConfig, EnsureCalibrationErrorKind<T::Error, F::Error>>
where
    D: CydDisplay<Error = T::Error>,
    T: CydTouchUncalibrated,
    F: FlashBlock,
    B: Button;
```

The exact error type may differ, but the helper should:

- load and return a valid saved configuration when present;
- run and internally retry the interactive flow when needed;
- display standard calibration text internally;
- save a validated new configuration;
- return only the resulting `CalibrationConfig`; and
- not consume the touch object.

The bundle implementation can then consume the intact uncalibrated CYD and apply the returned configuration:

```rust
let calibration_config = ensure_calibration_config(
    &mut self.display,
    &mut self.touch,
    calibration_flash_block,
    recalibration_button,
)
.await
.map_err(/* platform error mapping */)?;

Ok(CydEsp {
    display: self.display,
    touch: self.touch.calibrate(calibration_config),
})
```

Adjust this construction as needed to support the final-orientation requirement.

The existing `ensure_calibration` API may remain temporarily as a compatibility wrapper for unmigrated callers.

## Target DNS tester change

Make the source change in the Jinja template, not only in generated Rust:

```text
crates/device-envoy-examples-esp/examples/templates/dns_tester.rs.j2
```

Then regenerate the board examples.

The calibration-related portion of `inner_main` should become conceptually:

```rust
let [
    wifi_auto_flash_block,
    mut calibration_flash_block,
    mut orientation_flash_block,
] = FlashBlockEsp::new_array::<3>(p.FLASH)?;

let orientation = orientation_flash_block
    .load::<Orientation>()?
    .unwrap_or(Orientation::Landscape);

let button =
    DnsTesterButtonWatch::new(p.GPIO0, PressedTo::Ground, spawner).await?;

static CYD_STATIC: CydStaticEsp<STATUS_PIXEL_COUNT> = CydEsp::new_static();

let uncalibrated_cyd = CydEspUncalibrated::new(
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
    embedded_graphics::pixelcolor::Rgb888::new(10, 10, 12),
    embedded_graphics::pixelcolor::Rgb888::new(230, 230, 230),
    &DEFAULT_FONT,
    p.SPI3,
    p.GPIO25,
    p.GPIO32,
    p.GPIO39,
    p.GPIO33,
    p.GPIO36,
)?;
info!("CYD display and uncalibrated touch initialized");

let mut cyd = uncalibrated_cyd
    .into_calibrated(
        &mut calibration_flash_block,
        &mut *button,
    )
    .await?;
info!("CYD calibrated");
```

The rest of `inner_main` should continue from:

```rust
dns_tester_splash(cyd.display(), orientation)
```

and remain behaviorally unchanged in this task.

### Remove from this `inner_main`

Remove all of the following calibration orchestration:

- the explicit `CalibrationConfig` flash probe;
- `calibration_is_available`;
- `display_orientation_for_calibration`;
- direct `ensure_calibration` use;
- `EnsureCalibrationOutcome`;
- `was_saved()`;
- the button-release polling loop;
- the calibration-specific software reset;
- destructuring `CydEspUncalibrated` into display and touch; and
- `CydEsp::from_parts(display, touch)`.

### Import cleanup

The target generated file should no longer need calibration-related imports equivalent to:

```rust
CalibrationConfig
ensure_calibration
CydParts as _
display_orientation_for_calibration
```

It should import the new trait so method resolution is explicit:

```rust
use device_envoy_core::cyd::{
    Cyd as _,
    CydUncalibrated as _,
    display::Orientation,
};
```

Keep `CALIBRATION_MIN_PIXEL_COUNT` while it is still used to size `STATUS_PIXEL_COUNT`.

## Other concrete implementations

Add `CydUncalibrated` implementations for every other concrete uncalibrated CYD bundle that must satisfy the new trait, but do not implement their behavior in this change.

Each temporary method body must be exactly or substantially:

```rust
todo!("todo0000 fix this up")
```

Expected categories include:

- RP two-SPI uncalibrated CYD;
- ESP one-SPI uncalibrated CYD;
- RP one-SPI uncalibrated CYD;
- WASM uncalibrated CYD, if represented as a bundle type; and
- memory/test uncalibrated CYD, if represented as a bundle type.

Do not add this trait to raw-touch-only types such as `CydTouchUncalibratedEsp` or `CydTouchUncalibratedRp`.

### Missing one-SPI uncalibrated bundle types

Where a one-SPI backend currently constructs and calibrates everything inside its calibrated constructor, introduce an uncalibrated bundle type as scaffolding.

Its constructor must atomically create:

- the shared SPI bus;
- the display device handle; and
- the uncalibrated touch device handle.

For example:

```rust
pub struct CydEspOneSpiUncalibrated {
    display: CydDisplayEsp<SharedSpiDevice>,
    touch: CydTouchUncalibratedEsp<SharedSpiDevice>,
    // Any shared state or requested-orientation state needed by the bundle.
}
```

The display and touch objects must never be exposed as independently owned parts. The one-SPI uncalibrated bundle must not implement `CydParts`.

Its initial `CydUncalibrated::into_calibrated` body may be:

```rust
todo!("todo0000 fix this up")
```

This preserves the atomic resource model while postponing the full migration.

## Other generated `inner_main` functions

Do not refactor other generated launchers in this change.

Instead, update their source templates and add one `todo0000` comment near each comparable calibration/construction sequence.

For a manual split/calibrate/reassemble sequence, use wording like:

```rust
// todo0000 Consider replacing this manual calibration and reassembly
// with CydUncalibrated::into_calibrated, following the ESP DNS tester.
```

For a calibrated convenience constructor, use wording like:

```rust
// todo0000 Consider constructing the uncalibrated CYD explicitly and then
// calling CydUncalibrated::into_calibrated, following the ESP DNS tester.
```

Add the comment to templates or generator inputs, not just generated output.

Search at least for startup code using:

- `ensure_calibration`;
- `EnsureCalibrationOutcome`;
- `CydEsp::new`;
- `CydRp::new`;
- `CydEspOneSpi::new`;
- `CydRpOneSpi::new`;
- `from_parts`; and
- manual calibration flash probing.

Do not add duplicate comments when several of these occur in one startup sequence.

## Existing calibrated convenience constructors

Existing constructors such as `CydEsp::new`, `CydRp::new`, `CydEspOneSpi::new`, and `CydRpOneSpi::new` may remain temporarily for unmigrated callers.

For the real two-SPI ESP implementation, prefer rewriting `CydEsp::new` as a compatibility wrapper around:

```rust
CydEspUncalibrated::new(...)
    .into_calibrated(...)
    .await
```

Its return type should eventually become `Result<CydEsp>` rather than returning `EnsureCalibrationOutcome`.

Do not migrate all callers as part of this change.

## Tests

Add or update tests to cover the real two-SPI ESP bundle transition as far as practical without hardware.

At minimum, cover the shared/core behavior through memory-backed test doubles:

1. A valid configuration in flash is loaded and returned without interactive calibration.
2. Missing or corrupt calibration data enters the interactive flow.
3. Recoverable invalid calibration attempts remain inside the flow.
4. A validated calibration is saved.
5. Successful fresh calibration returns a calibrated bundle rather than an outcome requiring caller action.
6. A fatal display, touch, or flash failure returns `Err`.
7. The final returned bundle reports the requested application orientation.
8. The caller does not need to poll the button or reset after calibration.

Compile all temporary trait implementations containing:

```rust
todo!("todo0000 fix this up")
```

but do not route existing working examples through those stubs.

## Acceptance criteria

The change is complete when:

- The generated ESP two-SPI DNS tester constructs `CydEspUncalibrated` explicitly.
- It then calls `.into_calibrated(...).await?`.
- Its `inner_main` contains no caller-supplied calibration message.
- Its `inner_main` contains no calibration outcome handling.
- Its `inner_main` contains no calibration-specific button polling.
- Its `inner_main` contains no calibration-specific reset.
- Its `inner_main` contains no `CydEsp::from_parts`.
- `CydUncalibrated` is a public core trait.
- `CydEspUncalibrated` has a real `into_calibrated` implementation.
- Recoverable calibration failures retry internally.
- Success always returns a ready `CydEsp`.
- The returned CYD is in the requested application orientation.
- Other concrete uncalibrated CYD bundle implementations contain `todo!("todo0000 fix this up")`.
- Other generated startup templates contain targeted `todo0000` comments for later consideration.
- One-SPI scaffolding preserves atomic display-plus-uncalibrated-touch construction and does not introduce `CydParts`.
- Generated examples have been regenerated and formatting/checks pass.