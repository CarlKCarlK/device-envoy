# CYD Touch-Calibration Type-State API

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Replace the runtime `Option<CalibrationConfig>` calibration model with a
type-state API: a CYD device is either `Uncalibrated` or `Calibrated`, encoded
in its type, with by-value transitions between the two.

## Motivation

The current API has four defects the type system can eliminate:

1. **Silent no-events.** `CydTouch::read()` on an uncalibrated device returns
   `Ok(None)` forever — an app that forgets calibration compiles, runs, and
   just never sees touch.
2. **The apply footgun.** After the shared four-tap flow, the caller must
   remember `cyd.set_calibration(outcome.calibration_config())`. Forgetting it
   is defect 1.
3. **The witness hole.** `CalibratedCydEsp::clear_calibration()` clears the
   underlying device, but the wrapper keeps its copied config and continues
   reading calibrated events.
4. **Name collision.** `ensure_calibration` names both the shared async flow
   (core) and the per-device witness constructor (esp/rp).

## Design

### States

```rust
// device-envoy-core::cyd
pub struct Uncalibrated;                      // zero-sized
pub struct Calibrated(pub CalibrationConfig); // carries the affine map
```

Platform devices gain a state parameter:

```rust
pub struct CydEsp<State = Uncalibrated> { /* display, touch, state, ... */ }
// likewise CydRp<State>, CydWasm<State>
```

The fields `calibration_config: Option<CalibrationConfig>` and the methods
`set_calibration`, `clear_calibration`, and the device-level
`ensure_calibration` (witness constructor) are deleted, along with
`CalibratedCydEsp` / `CalibratedCydRp` and `CydError::CalibrationUnavailable`.

Transitions consume the device:

```rust
impl CydEsp<Uncalibrated> {
    pub fn calibrate(self, config: CalibrationConfig) -> CydEsp<Calibrated>;
}
impl CydEsp<Calibrated> {
    pub fn calibration_config(&self) -> CalibrationConfig;
    pub fn recalibrate(self) -> CydEsp<Uncalibrated>;
}
```

Because going uncalibrated consumes the calibrated device, no stale calibrated
handle can outlive the transition — the borrow checker enforces what the old
witness API only implied (fixes defect 3).

### Trait split

Today's `Cyd` trait bundles display + calibrated touch via `parts()`. Split
out a display-only capability so the calibration flow can drive an
uncalibrated screen:

```rust
/// Display-only capability. Implemented by both states.
pub trait CydScreen {
    type Error;
    type Display<'a>: CydDisplay<Error = Self::Error>
    where
        Self: 'a;
    fn display(&mut self) -> Self::Display<'_>;
}

/// Full device: display + calibrated touch. Implemented only by Calibrated.
pub trait Cyd: CydScreen {
    type Touch<'a>: CydTouch<Error = Self::Error>
    where
        Self: 'a;
    fn parts(&mut self) -> (Self::Display<'_>, Self::Touch<'_>);
}

/// The upward transition. Implemented only by Uncalibrated.
pub trait CydUncalibrated:
    CydScreen + CydRawTouch<Error = <Self as CydScreen>::Error> + Sized
{
    type Calibrated: CydCalibrated<Uncalibrated = Self>;
    fn calibrate(self, config: CalibrationConfig) -> Self::Calibrated;
}

/// The downward transition. Implemented only by Calibrated.
pub trait CydCalibrated: Cyd + Sized {
    type Uncalibrated: CydUncalibrated<Calibrated = Self>;
    fn recalibrate(self) -> Self::Uncalibrated;
    fn calibration_config(&self) -> CalibrationConfig;
}
```

Notes:

- The mutually recursive associated types (`U::Calibrated::Uncalibrated == U`)
  are legal Rust and give generic code a round trip at the same type.
- `CydRawTouch` moves to the uncalibrated side only. Calibrated apps can no
  longer read raw samples — an intentional tightening; the raw path exists
  solely for the calibration flow.
- The touch part's `read()` always has a config (carried by copy from
  `Calibrated`), so the `Ok(None)`-when-uncalibrated branch disappears
  (fixes defect 1).

### The shared flow becomes the gate

`ensure_calibration` (core, `cyd::touch::calibration`) consumes the
uncalibrated device and returns the calibrated one, making defect 2
unrepresentable. With the device-level witness constructor deleted, the name
collision (defect 4) also resolves — the free function keeps the name.

```rust
pub async fn ensure_calibration<U, F, R>(
    cyd: U,                              // consumed
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
) -> Result<(U::Calibrated, EnsureCalibrationOutcome), EnsureCalibrationError<U, F::Error>>
where
    U: CydUncalibrated,
    F: FlashBlock,
    R: Button;
// ensure_calibration_with_settings gains the same shape (wasm needs the
// larger verify-frame budget, as today).
```

**Errors must hand the device back**, or a mid-flow SPI hiccup permanently
strands the hardware:

```rust
pub struct EnsureCalibrationError<U: CydUncalibrated, FlashError> {
    pub cyd: U, // returned so the caller can retry or fall back to display-only
    pub kind: EnsureCalibrationErrorKind<<U as CydScreen>::Error, FlashError>,
}

pub enum EnsureCalibrationErrorKind<DeviceError, FlashError> {
    Device(DeviceError),
    Flash(FlashError),
}
```

Call sites simplify:

```rust
let (mut cyd, _outcome) =
    ensure_calibration(cyd, &mut flash, &mut button, Some("Touch calibrated"))
        .await
        .map_err(|error| error.kind)?; // or keep `error.cyd` to retry
// cyd: CydEsp<Calibrated>
```

### Runtime recalibration round trip

Generic code can go down and come back up at the same type, and failure
recovery is total because `CalibrationConfig` is `Copy`:

```rust
async fn rerun_calibration<C, F, R>(
    cyd: C,
    flash: &mut F,
    button: &mut R,
) -> Result<C, EnsureCalibrationError<C::Uncalibrated, F::Error>>
where
    C: CydCalibrated,
    F: FlashBlock,
    R: Button,
{
    let old_config = cyd.calibration_config(); // snapshot for recovery
    let uncalibrated = cyd.recalibrate();      // C -> C::Uncalibrated
    // clear flash, then:
    let (calibrated, _outcome) =
        ensure_calibration(uncalibrated, flash, button, None).await?;
    Ok(calibrated)                             // C::Uncalibrated::Calibrated == C
    // On Err, the caller can restore the old state without re-running the
    // flow: `error.cyd.calibrate(old_config)`.
}
```

Constraint carried over from the old design: the app must own the device at
the recalibration moment. The per-frame `parts()` borrow pattern is
unaffected; recalibration happens between frames where the borrows are dead.

## Display-only devices (deferred — wait and see)

`CydEsp::new_display_only` currently returns the same `CydEsp` with
`touch: None`, so `touch` stays `Option<CydTouchEsp>` internally and
`CydError::TouchUnavailable` survives for now. The eventual clean version
makes `new_display_only` return a distinct display-only device type
implementing only `CydScreen`, letting `CydEsp<State>` always own touch
hardware and deleting the `Option` and `TouchUnavailable`. Do this only if
the `Option` proves annoying in practice; it is not required for the
type-state core.

## Deleted API surface

- `CydEsp::set_calibration`, `clear_calibration`, `calibration_config()`
  (the `Option` accessor), device-level `ensure_calibration` (same for rp,
  wasm)
- `CalibratedCydEsp`, `CalibratedCydRp`
- `CydError::CalibrationUnavailable`
- The `Ok(None)`-when-uncalibrated branch of `CydTouch::read`
- `CydRawTouch` impls on calibrated devices

## Migration checklist

- [ ] core: add `Uncalibrated` / `Calibrated`, `CydScreen`, `CydUncalibrated`,
      `CydCalibrated`; rebase `Cyd` on `CydScreen`
- [ ] core: rewrite `ensure_calibration` / `ensure_calibration_with_settings`
      to consume/return devices; driver draws via `CydScreen::display()`
      instead of `parts()`
- [ ] core: restructure `EnsureCalibrationError` to carry the device back
- [ ] esp: `CydEsp<State>`, transitions, trait impls per state
- [ ] rp: `CydRp<State>`, same
- [ ] core wasm: `CydWasm<State>`, same (keep the larger verify-frame budget
      via `ensure_calibration_with_settings`)
- [ ] examples: all `cyd_touch_paint` variants switch to the consume/return
      call shape; `cyd_tiles` (display-only) unaffected beyond type names
- [ ] doctests: update the `DemoCyd` boilerplate in the driver doc example to
      the new trait split
- [ ] linkage-blaze: update CYD call sites and re-run `just check-all` in both
      repos

No backwards-compatibility shims or type aliases — refactor call sites
directly (per AGENTS.md).

## Open questions

- Does anything need raw touch *after* calibration (diagnostics?) — if yes,
  `CydRawTouch` stays implemented for both states instead of uncalibrated
  only.
- Part-trait naming: `CydDisplay` (the `parts()` display half) vs. a future
  display-only *device* — rename part traits (`CydDisplayPart`?) only if the
  display-only device split from the deferred section happens.
