# CYD Owned Parts: Always-Calibrated Touch as a Distinct Type

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Supersedes `CYD_TOUCH_TYPESTATE_SPEC.md` (implemented, then reconsidered): the
device-level `Cyd<State>` type-state is replaced by **owned display and touch
components**, with calibration state expressed as **two distinct touch types**
rather than a generic marker parameter.

## Motivation

The device-level type-state (`CydEsp<Uncalibrated>` / `CydEsp<Calibrated>`)
works, but it puts the state on the wrong noun and pays for it everywhere:

1. Calibration is a property of **touch**, yet the whole device changes type.
   Display-only code drags a meaningless state parameter around
   (`new_display_only` returns an "uncalibrated" device that can never be
   calibrated), and `touch: Option<...>` + `CydError::TouchUnavailable`
   survive as runtime artifacts.
2. `parts()` hands out **borrowed** halves, and a `&mut Touch` can never
   change type — through a `&mut T` you can swap values of the same type
   (`mem::replace`), never the type itself. That borrow is what forced the
   state up onto the device in the first place.
3. `ensure_calibration` must consume and return the **entire device**, so its
   error type smuggles the whole device back to the caller.
4. Generic markers (`CydEsp<Calibrated>`) are harder to read than plain named
   types, and the supporting trait lattice (`CydScreen`, `Cyd`,
   `CydUncalibrated`, `CydCalibrated`) is large.

The fix is structural: on every backend the display and touch halves share
**nothing** (separate SPI buses on ESP/RP; `Rc`-cloned sources on wasm;
`RefCell`-backed scripts on the memory mock). The device struct is just a
container, so construction can yield the two owned components directly.

## Design

### Core traits

`CydDisplay` keeps its exact current surface (frames, tiles, fills, colors) —
it simply becomes the trait of an **owned display component** instead of a
borrowed part. `CydScreen`, `Cyd`, `CydUncalibrated`, `CydCalibrated`, and the
`Uncalibrated` / `Calibrated` markers are deleted. `CydRawTouch` is renamed
and absorbed into the new pair:

```rust
pub trait CydTouchUncalibrated: Sized {
    type Error;
    type Calibrated: CydTouch<Error = Self::Error, Uncalibrated = Self>;

    /// Read the next raw controller sample, if any.
    fn read_raw_touch_event(&mut self) -> Result<Option<RawTouchEvent>, Self::Error>;

    /// Apply `calibration_config`, becoming a calibrated touch.
    fn calibrate(self, calibration_config: CalibrationConfig) -> Self::Calibrated;
}

pub trait CydTouch: Sized {
    type Error;
    type Uncalibrated: CydTouchUncalibrated<Error = Self::Error, Calibrated = Self>;

    /// Read the next calibrated screen-space touch event, if any.
    fn read(&mut self) -> Result<Option<TouchEvent>, Self::Error>;

    fn calibration_config(&self) -> CalibrationConfig;

    /// Discard the calibration, becoming an uncalibrated touch.
    fn decalibrate(self) -> Self::Uncalibrated;
}
```

The mutually recursive associated types give generic code the round trip at
the same type (`T::Uncalibrated::Calibrated == T`). Because transitions
consume `self`, no stale calibrated handle can outlive a `decalibrate` — the
guarantee the old borrowed-parts design could not express.

"Always calibrated" is enforced **by construction**: the only ways to obtain
a `CydTouch` value are `CydTouchUncalibrated::calibrate` and the
`ensure_calibration` flow.

### Platform types: two distinct structs, no generics

Each platform defines plainly named types (readability over marker generics):

```rust
pub struct CydDisplayEsp { /* display driver, pixel buffer, colors, font */ }
pub struct CydTouchUncalibratedEsp { /* XPT2046 SPI device */ }
pub struct CydTouchEsp {
    raw: CydTouchUncalibratedEsp,
    calibration_config: CalibrationConfig,
}
// likewise ...Rp, ...Wasm
```

`CydEsp` survives, but as a **plain two-field bundle, not a device with
methods to route through** — it is what you get back when you already have a
saved `CalibrationConfig` (the common case: boot with flash already
populated). A sibling `CydEspUncalibrated` is the bundle for the pre-
calibration case, and `CydDisplayEsp` alone covers touch-free construction.
All three are ordinary structs; no trait lattice, no generic state parameter:

```rust
pub struct CydEsp {
    pub display: CydDisplayEsp,
    pub touch: CydTouchEsp,
}

pub struct CydEspUncalibrated {
    pub display: CydDisplayEsp,
    pub touch: CydTouchUncalibratedEsp,
}

impl CydEsp {
    pub const SCREEN_PIXELS: usize;
    pub const fn new_static<const PIXEL_COUNT: usize>() -> CydStaticEsp<PIXEL_COUNT>;

    /// Construct with an already-known calibration (the common boot path:
    /// flash already holds a saved `CalibrationConfig`).
    pub fn new(/* statics, display pins, colors, font, touch pins */, calibration_config: CalibrationConfig)
        -> Result<Self, Error>;
}

impl CydEspUncalibrated {
    /// Construct without a calibration yet; hand `touch` to `ensure_calibration`.
    pub fn new(/* statics, display pins, colors, font, touch pins */)
        -> Result<Self, Error>;
}

impl CydDisplayEsp {
    /// Construct just the display (no touch hardware used).
    pub fn new(/* statics, display pins, colors, font */) -> Result<Self, Error>;
}
```

Both bundle structs derive nothing special — an app that wants the pair
destructures it (`let CydEsp { display, touch } = cyd;` or the field access
directly); an app that wants the current call-site shape can still write
`let (display, touch) = (cyd.display, cyd.touch);`. Deleted with this change:
the `touch: Option<CydTouchEsp>` field and the display-only-device-with-
phantom-state awkwardness. Display-only apps (ballet, clock, skeleton-clock,
cyd_tiles) construct a `CydDisplayEsp` and nothing else — they already hold
only `display`, so their diffs are one line.

`CydError` is deleted outright and folded into `device_envoy_esp::Error`: its
variants (`DisplayInit`, `TouchInit`, `DisplayFlush`) become new `Error`
variants directly, `TouchUnavailable` is dropped (touch is always present
once constructed), and `Flash(crate::Error)` disappears as a wrapper since
the outer type *is* now `Error`. Every CYD operation returns
`device_envoy_esp::Result<T>` — one error type for the whole crate, and
consuming apps drop their `From<CydError> for MainError` impl entirely.

### The calibration flow borrows the display, consumes the touch

```rust
pub async fn ensure_calibration<D, T, F, R>(
    display: &mut D,
    touch: T,                          // consumed
    calibration_flash_block: &mut F,
    recalibration_button: &mut R,
    confirmed_message: Option<&str>,
) -> Result<(T::Calibrated, EnsureCalibrationOutcome), EnsureCalibrationError<T, F::Error>>
where
    D: CydDisplay,
    T: CydTouchUncalibrated<Error = D::Error>,
    F: FlashBlock,
    R: Button;
// ensure_calibration_with_settings keeps the same shape (wasm still passes
// its larger verify-frame budget).
```

The error only has to hand back the small touch value — the display was never
at risk because it stays borrowed:

```rust
pub struct EnsureCalibrationError<T: CydTouchUncalibrated, FlashError> {
    pub touch: T,
    pub kind: EnsureCalibrationErrorKind<T::Error, FlashError>,
}
```

Failure recovery stays total: `CalibrationConfig` is `Copy`, so a caller that
snapshotted the old config can restore with `error.touch.calibrate(old_config)`.

### Generic apps take the components they use

```rust
pub async fn ballet<D: CydDisplay>(display: &mut D) -> ...;

pub async fn armatron<D, T, R>(
    display: &mut D,
    touch: &mut T,
    recalibration_button: &mut R,
) -> Result<ArmatronExit, Error<D::Error>>
where
    D: CydDisplay,
    T: CydTouch<Error = D::Error>,
    R: Button;
```

The whole-device `Cyd` trait is deleted — apps take the `D`/`T` components
they need directly, generic over `CydDisplay`/`CydTouch`. (`CydEsp` itself
survives as a plain bundle struct, per the factory shapes above; it is not a
trait, just a convenience for the common already-calibrated boot path.)

End-to-end ESP shape, boot with no saved calibration yet:

```rust
static CYD_STATIC: CydStaticEsp<{ CydEsp::SCREEN_PIXELS }> = CydEsp::new_static();
let CydEspUncalibrated { mut display, touch } = CydEspUncalibrated::new(&CYD_STATIC, /* pins... */)?;

let (mut touch, outcome) =
    ensure_calibration(&mut display, touch, &mut flash, &mut button, Some("rebooting")).await?;
if outcome.was_saved() {
    software_reset(); // re-enter the canonical boot path with the saved config
}

match armatron(&mut display, &mut touch, &mut button).await? {
    ArmatronExit::CalibrationRequested => {
        flash.clear()?;
        software_reset();
    }
}
```

Boot with a saved calibration already in flash skips the flow entirely:

```rust
let CydEsp { mut display, mut touch } =
    CydEsp::new(&CYD_STATIC, /* pins... */, saved_calibration_config)?;
armatron(&mut display, &mut touch, &mut button).await?;
```

### Recalibration doctrine

Two patterns, both plain consequences of the design — no extra machinery:

- **ESP (and RP): clear flash + software reset.** Calibration is persisted
  boot configuration; reset re-enters the one canonical boot path that
  already runs `ensure_calibration`. This is what armatron does today and it
  stays the endorsed answer on hardware.
- **wasm (no reset available): owned round trip.** The app loop calls
  `touch.decalibrate()`, reruns `ensure_calibration(&mut display, ...)`, and
  receives a fresh calibrated touch — the display is reused, not rebuilt.

### `CydMemory`: a test harness, born calibrated

`CydMemory` keeps existing as the one "whole device" — a **harness** whose
internals are already `Rc`/`RefCell`-shared, so it can hand out owned parts
while retaining its inspection surface:

```rust
let mut cyd = CydMemory::new(size, background, foreground, &FONT);
let (mut display, mut touch) = cyd.parts();   // owned; state shared with the harness
cyd.push_touch_event(TouchEvent::Down { .. }); // harness keeps scripting + assertions:
cyd.set_frame_budget(1);                       // pixel(), flush_count(),
// run app...                                  // last_flush_rectangle(), script_raw_frames()
assert_eq!(cyd.pixel(160, 120), Rgb565::RED);
```

- `parts()` returns `(CydDisplayMemory, CydTouchMemory)` with the touch
  **already calibrated** (identity mapping) — app tests never mention
  `CalibrationConfig`. This finally removes the identity-calibrate ceremony
  from every doctest and app test.
- Calibration-driver tests call `touch.decalibrate()` (or a
  `parts_uncalibrated()` convenience) to get the raw-script-reading
  `CydTouchUncalibratedMemory`.
- No state parameter anywhere on the harness.

The `Cyd` trait doc example moves to `CydDisplay`/`CydTouch` (or a module-level
example on `cyd`), and its hidden setup shrinks accordingly.

## Deleted API surface

- Traits: `Cyd`, `CydScreen`, `CydUncalibrated`, `CydCalibrated`,
  `CydRawTouch` (renamed/absorbed into `CydTouchUncalibrated`)
- Markers: `Uncalibrated`, `Calibrated`
- Device state parameters: `CydEsp<State>`, `CydRp<State>`, `CydWasm<State>`,
  `CydMemory<State>`
- `touch: Option<...>` fields
- `parts()`-as-borrows (memory keeps a `parts()` that yields owned handles)
- Device-carrying `EnsureCalibrationError { cyd, .. }` (now carries only the
  touch)
- `CydError` per platform crate — folded into that crate's top-level `Error`
  (`TouchUnavailable` dropped entirely; `Flash`/`DisplayInit`/`TouchInit`/
  `DisplayFlush` become `Error` variants directly)

## Migration checklist

- [ ] core `cyd.rs`: delete the device trait lattice; keep `CydDisplay`
      surface as-is; move the primary doc example onto `CydDisplay`/`CydTouch`
- [ ] core `cyd/touch.rs`: define `CydTouch` / `CydTouchUncalibrated` with the
      mutual associated types; delete `CydRawTouch`
- [ ] core `driver.rs`: `ensure_calibration(display, touch, ...)` shape; error
      carries `touch` back; update the demo-impl doctest boilerplate
- [ ] core `memory.rs`: harness with owned `Rc`-shared parts; identity-
      calibrated by default; migrate its test module
- [ ] core `wasm.rs`: `CydDisplayWasm` / `CydTouchWasm` /
      `CydTouchUncalibratedWasm`; keep the larger verify budget path
- [ ] esp: `CydDisplayEsp` (+ its own constructor), `CydTouchEsp`,
      `CydTouchUncalibratedEsp`; `CydEsp` / `CydEspUncalibrated` bundle structs
      per the factory shapes above; fold `CydError` into `Error`, delete
      `Option`/`TouchUnavailable`
- [ ] rp: same split (`CydRp` / `CydRpUncalibrated`)
- [ ] esp examples + templates (`cyd_touch_paint.rs.j2`, `cyd_tiles.rs.j2`),
      then `cargo xtask generate-board-examples`; rp example
- [ ] linkage-blaze: armatron (esp + wasm + example-core + tests) — drop
      `From<CydError> for MainError`; ballet, clock, skeleton-clock call sites
- [ ] `cargo check-all` (device-envoy) and `just check-all` (linkage-blaze)
      both green

No backwards-compatibility shims or type aliases — refactor call sites
directly (per AGENTS.md).

## Resolved decisions

- **Factory shape**: `CydEsp` / `CydEspUncalibrated` are plain two-field
  bundle structs (not traits, no generic state parameter); `CydEsp::new`
  requires an already-known `CalibrationConfig`, `CydEspUncalibrated::new`
  does not, `CydDisplayEsp::new` skips touch entirely.
- **`decalibrate()`**: kept on all platforms, including hardware
  (`CydTouchEsp`, `CydTouchRp`), for trait symmetry — even though the
  endorsed hardware recalibration path is clear-flash-and-reset, not this
  method.
- **Error enums**: `CydError` is deleted; its variants fold into each
  platform crate's single top-level `Error`, not split into per-component
  enums.
