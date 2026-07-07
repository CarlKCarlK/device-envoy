# CYD Bundle Follow-Ups: One-Call Construction and a Thin `Cyd` Trait

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Follow-up to `CYD_OWNED_PARTS_SPEC.md` (implemented). Review of that
implementation found three gaps between what landed and where the API should
end up:

1. **`CydEsp::new(..., calibration_config)` is dead code.** No call site
   constructs a `CydEsp` (or `CydRp`), because the flash load lives *inside*
   `ensure_calibration` — no app ever has a `CalibrationConfig` in hand
   before construction. Every app boots via `CydEspUncalibrated::new` →
   destructure → `ensure_calibration` → carry loose parts forever; the
   calibrated bundle never appears.
2. **Generic core code lost its whole-device abstraction.** `armatron` takes
   `(&mut impl CydDisplay, &mut impl CydTouch, ...)`; there is no way to
   pass "a Cyd".
3. **`CydError` was not folded into the crate `Error`** (resolved decision
   #3 of the previous spec, unimplemented).

This spec fixes all three. Goal restated: *users just create a Cyd, and
generic code just accepts a Cyd* — with the owned-parts machinery remaining
underneath as the calibration/construction layer and the escape hatch.

## F1: `CydEsp::new` absorbs `ensure_calibration`

Replace the dead config-taking constructor. `CydEsp::new` becomes the one
call most apps make: async, loads the calibration from flash or runs the
on-screen four-tap flow (saving the result), and returns a ready calibrated
bundle. The uncalibrated state exists only inside the constructor.

```rust
impl CydEsp {
    /// Construct a ready, calibrated CYD: loads the calibration from flash,
    /// or runs the on-screen four-tap flow and saves the result.
    /// Internally: `CydEspUncalibrated::new(...)` + `ensure_calibration(...)`.
    ///
    /// Returns the outcome alongside the device so callers can keep the
    /// reset-after-fresh-save behavior (`outcome.was_saved()`).
    pub async fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        /* display pins, orientation, background, foreground, font */
        /* touch pins */
        calibration_flash_block: &mut FlashBlockEsp,
        recalibration_button: &mut impl Button,
        confirmed_message: Option<&str>,
    ) -> Result<(Self, EnsureCalibrationOutcome), Error>;
}
// likewise CydRp::new with FlashBlockRp
```

- The flash block is the concrete platform type (`FlashBlockEsp`), which
  collapses the error story: with F3, both the device and flash sides of
  `EnsureCalibrationErrorKind` are already `Error`, so the constructor
  returns plain `crate::Error` — no touch-carrying error type at this level.
- **Trade-off (accepted):** if the flow errors mid-way, the ESP pins were
  consumed and the hardware cannot be rebuilt in place — unlike the
  parts-level flow, whose error hands the touch back. On embedded the
  realistic response to a boot-time failure is panic/reset. Apps that need
  mid-flow recovery use the `CydEspUncalibrated` escape hatch, which stays.
- `CydEspUncalibrated::new`, `CydDisplayEsp::new`, and the parts-level
  `ensure_calibration` are unchanged — they are the layer `CydEsp::new` is
  built from, and remain public for custom calibration UIs.
- wasm gets no flow-absorbing constructor: the browser build already runs a
  reconstruct-per-calibration loop with `ensure_calibration_with_settings`
  and its larger verify budget; see F2 for how it bundles the result.

## F2: restore `Cyd` as a thin trait over stored parts

The old `Cyd` trait needed GATs because `parts()` *manufactured* part values
per call. Under owned parts, implementors **store** the two components as
fields, so the trait returns plain disjoint `&mut` borrows — no GATs, no
lifetime gymnastics:

```rust
// device-envoy-core::cyd
pub trait Cyd {
    type Error;
    type Display: CydDisplay<Error = Self::Error>;
    type Touch: CydTouch<Error = Self::Error>;

    /// Borrow both halves at once (disjoint field borrows).
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    fn display(&mut self) -> &mut Self::Display {
        self.parts().0
    }

    fn touch(&mut self) -> &mut Self::Touch {
        self.parts().1
    }
}
```

Implementors:

- **`CydEsp` / `CydRp`** — one-liners returning field refs.
- **`CydBundle<D, T>`** (new, in core) — a generic two-field struct with
  public fields and a blanket impl, so *any* parts pair can be bundled after
  a custom flow:

  ```rust
  pub struct CydBundle<D, T> {
      pub display: D,
      pub touch: T,
  }

  impl<D, T> Cyd for CydBundle<D, T>
  where
      D: CydDisplay,
      T: CydTouch<Error = D::Error>,
  { /* field refs */ }
  ```

- **`CydMemory` / `CydWasm` harnesses** — store one `display` and one
  (identity-calibrated) `touch` as fields at construction instead of minting
  them per call; `Cyd::parts` returns refs to those. The inherent by-value
  `parts()` is deleted (the trait method takes its name); the by-value
  `display()` clone accessor and `parts_uncalibrated()` stay for driver
  tests and extra handles. Inspection surface (`pixel()`, `flush_count()`,
  `push_touch_event()`, `script_raw_frames()`) is unchanged.

Generic runtime code goes back to a single parameter:

```rust
pub async fn armatron<C: Cyd, R: Button>(
    cyd: &mut C,
    recalibration_button: &mut R,
) -> Result<ArmatronExit, Error<C::Error>> {
    let (display, touch) = cyd.parts();
    // ... loop unchanged
}
// ballet/clock/skeleton-clock stay display-only: `&mut impl CydDisplay`.
```

Call sites:

```rust
// esp (with F1): one constructor, one Cyd, pass it whole
let (mut cyd, outcome) =
    CydEsp::new(&CYD_STATIC, /* pins... */, &mut flash, &mut button, Some("rebooting")).await?;
if outcome.was_saved() {
    software_reset();
}
armatron(&mut cyd, &mut button).await?;

// wasm: parts-level flow (real config from localStorage), then bundle
let (mut display, touch_uncalibrated) = source_cyd.parts_uncalibrated();
let (touch, _outcome) = ensure_calibration_with_settings(
    &mut display, touch_uncalibrated, &mut flash, &mut button, None, settings,
).await /* error handling */;
let mut cyd = CydBundle { display, touch };
armatron(&mut cyd, &mut button).await;

// tests: pass the harness directly
armatron(&mut memory_cyd, &mut memory_button)
```

Division of labor after F2:

- **Construction & calibration**: parts-based (ownership transitions live
  here; `ensure_calibration` borrows the display, consumes/returns touch).
- **Generic runtime code**: `&mut impl Cyd`.
- **Display-only code**: `&mut impl CydDisplay`, unchanged.

## F3: fold `CydError` into the crate `Error`

Carried over from the previous spec's resolved decision #3. `CydError` is
deleted in both esp and rp:

- `DisplayInit(CydDisplayEspInitError)`, `TouchInit(CydTouchEspInitError)`,
  and `DisplayFlush(CydDisplayEspFlushError)` become variants of
  `device_envoy_esp::Error` directly (same pattern for rp).
- The `Flash(crate::Error)` wrapper variant disappears — the outer type *is*
  now `Error`.
- Every CYD operation returns `device_envoy_esp::Result<T>`; consuming apps
  (armatron esp binary and friends) delete their `From<CydError> for
  MainError` boilerplate.

## Migration checklist

- [ ] core `cyd.rs`: add the `Cyd` trait and `CydBundle`
- [ ] core `memory.rs` / `wasm.rs`: harnesses store `display`/`touch` fields,
      implement `Cyd`, drop the inherent by-value `parts()`
- [ ] esp: replace `CydEsp::new(..., calibration_config)` with the
      flow-absorbing async constructor; fold `CydError` into `Error`
- [ ] rp: same two changes (`CydRp::new`, `CydError` fold)
- [ ] core driver/doc examples: update the demo boilerplate to the new
      shapes
- [ ] esp templates (`cyd_touch_paint.rs.j2`), then
      `cargo xtask generate-board-examples`; rp `cyd_touch_paint.rs`
- [ ] linkage-blaze: armatron generic fn back to `C: Cyd`; esp binary uses
      `CydEsp::new` and drops `From<CydError>`; wasm binary bundles with
      `CydBundle`; memory tests pass the harness directly
- [ ] `cargo check-all` (device-envoy) and `just check-all` (linkage-blaze)
      both green

No backwards-compatibility shims or type aliases — refactor call sites
directly (per AGENTS.md).
