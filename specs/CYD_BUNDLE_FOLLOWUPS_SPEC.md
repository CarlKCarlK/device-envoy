# CYD Bundle Follow-Ups: Restore `Cyd` for Generic Code

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Follow-up to `CYD_OWNED_PARTS_SPEC.md` (implemented). That refactor improved
the ownership model around calibration, but it overshot on API shape:
construction and calibration became cleanly parts-based, while generic runtime
code lost the ability to simply "take a Cyd".

This spec restores that missing layer without undoing the owned-parts work.

## Summary

The owned-parts refactor got one major thing right:

- calibration is a property of touch, not of the whole device
- `ensure_calibration` should borrow display and consume/return touch
- display-only apps should be able to construct just a display

But it also introduced three follow-up problems:

1. `CydEsp::new(..., calibration_config)` and `CydRp::new(..., calibration_config)`
   are not the real construction path. Apps do not have a
   `CalibrationConfig` before boot; the flash load and calibration flow live
   inside `ensure_calibration`.
2. Generic app code can no longer take `&mut impl Cyd`; it must take loose
   `display` and `touch` values separately.
3. `CydError` survived as a parallel error type in esp/rp, even though the
   previous spec explicitly intended to fold those variants into the crate
   `Error`.

The intended end state is:

- generic runtime code takes `&mut impl Cyd`
- construction/calibration remains parts-based internally
- constructors remain inherent methods on concrete platform types such as
  `CydEsp::new(...).await`, `CydRp::new(...).await`, `CydWasm::new(...)`, and
  `CydMemory::new(...)`
- the parts APIs remain public as the lower-level escape hatch

## Design Goal

Separate the two layers cleanly:

- **Construction/calibration layer:** owned `display` and `touch` parts
- **Generic runtime layer:** a thin bundled `Cyd` trait over stored parts

There is **no** platform-independent `Cyd::new`. The shared `Cyd`
abstraction exists for generic app code only after a concrete platform type
has already been constructed.

The mistake in the implemented pass was treating the construction layer as if
it had to also be the only generic runtime abstraction. It does not.

## Part 1: platform constructors stay concrete and absorb calibration

The top-level platform constructors should match the real boot path, but they
stay on the concrete platform types.

Replace:

```rust
pub fn new(..., calibration_config: CalibrationConfig) -> Result<Self, Error>
```

with an async constructor that performs the same sequence apps currently
write manually:

1. construct `Cyd*Uncalibrated`
2. run `ensure_calibration`
3. return the calibrated bundle plus `EnsureCalibrationOutcome`

Shape:

```rust
impl CydEsp {
    pub async fn new<const PIXEL_COUNT: usize>(
        statics: &'static CydStaticEsp<PIXEL_COUNT>,
        /* display pins, orientation, colors, font */
        /* touch pins */
        calibration_flash_block: &mut FlashBlockEsp,
        recalibration_button: &mut impl Button,
        confirmed_message: Option<&str>,
    ) -> Result<(Self, EnsureCalibrationOutcome), Error>;
}
```

Likewise for `CydRp`.

`CydWasm::new(...)` and `CydMemory::new(...)` remain synchronous because they
already construct ready-to-use local/test devices without embedded flash boot
flow.

Notes:

- `CydEspUncalibrated::new`, `CydRpUncalibrated::new`, `CydDisplayEsp::new`,
  `CydDisplayRp::new`, and `ensure_calibration` all stay public.
- The async top-level constructor is the **default path**, not the only path.
- This keeps the reset-after-fresh-save flow straightforward:

```rust
let (mut cyd, calibration_outcome) =
    CydEsp::new(&CYD_STATIC, /* pins... */, &mut flash, &mut button, Some("rebooting")).await?;
if calibration_outcome.was_saved() {
    software_reset();
}
```

Trade-off:

- if boot-time calibration fails inside `CydEsp::new`, the consumed platform
  peripherals are gone, so this constructor cannot hand the device back
  piece-by-piece
- that is acceptable for the top-level embedded boot path
- callers that need recovery or a custom calibration UI use
  `CydEspUncalibrated::new` / `CydRpUncalibrated::new` directly

## Part 2: restore a thin `Cyd` trait in core

Generic runtime code should be able to say "give me a Cyd" again.

The restored trait is intentionally thin. It does not own calibration state,
does not model type-state, and does not manufacture temporary part wrappers.
It simply borrows the two stored parts of an already-constructed bundle.

```rust
pub trait Cyd {
    type Error;
    type Display: CydDisplay<Error = Self::Error>;
    type Touch: CydTouch<Error = Self::Error>;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    fn display(&mut self) -> &mut Self::Display {
        self.parts().0
    }

    fn touch(&mut self) -> &mut Self::Touch {
        self.parts().1
    }
}
```

This trait is different from the old one in one important way:

- the old trait needed a larger lattice because the parts were effectively
  synthesized through borrowed wrappers
- the new trait is just a borrowing facade over already-owned fields

That means:

- no `CydScreen`
- no `CydUncalibrated`
- no `CydCalibrated`
- no state-marker parameter on the whole device
- no GAT-heavy borrowed-part manufacturing API

## Part 3: platform bundles implement `Cyd`

`CydEsp` and `CydRp` remain real structs with real `display` and `touch`
fields. They should implement the restored `Cyd` trait directly:

```rust
impl Cyd for CydEsp {
    type Error = Error;
    type Display = CydDisplayEsp;
    type Touch = CydTouchEsp;

    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch) {
        (&mut self.display, &mut self.touch)
    }
}
```

Likewise for RP.

This gives the API its missing shape back:

- apps can destructure and use parts when they want to
- generic code can take `&mut impl Cyd` when that is the natural abstraction

## Part 4: memory/wasm harnesses implement `Cyd`

The harnesses should stop being "special non-`Cyd` things" for generic code.

Target shape:

- `CydMemory` stores one calibrated display part and one calibrated touch part
- `CydWasm` stores one calibrated display part and one calibrated touch part
- both implement the restored `Cyd` trait

Important nuance:

- keep the extra harness/testing surface
- keep lower-level helpers like `parts_uncalibrated()` where they are useful
- only remove the old inherent by-value `parts()` if it conflicts with the
  trait method name

This means:

- app tests can pass `&mut CydMemory` directly to generic `Cyd` consumers
- wasm can still do custom calibration work and then update its stored touch
  before entering generic `Cyd`-consuming code

## Part 5: fold `CydError` into crate `Error`

This follow-up should finish the error-model simplification that the owned
parts spec intended but the implementation did not complete.

Delete `CydError` from esp and rp.

Move its variants into the crate-level `Error`:

- `DisplayInit(...)`
- `TouchInit(...)`
- `DisplayFlush(...)`

Delete:

- `Flash(crate::Error)` wrapper variant
- `TouchUnavailable`

Rationale:

- `TouchUnavailable` belonged to the old optional-touch runtime model and is
  invalid under owned calibrated touch values
- a separate `CydError` now adds conversion noise without carrying distinct
  meaning

After this, CYD APIs in esp/rp return the crate `Result<T>` directly.

## Generic app signatures after this spec

`armatron` is the main consumer that should change back:

```rust
pub async fn armatron<C, R>(
    cyd: &mut C,
    recalibration_button: &mut R,
) -> Result<ArmatronExit, Error<C::Error>>
where
    C: Cyd,
    R: Button,
{
    let (display, touch) = cyd.parts();
    // loop body remains effectively the same
}
```

Display-only apps stay as they are:

```rust
pub async fn ballet<D: CydDisplay>(display: &mut D) -> ...;
pub async fn clock<D: CydDisplay>(display: &mut D, ...) -> ...;
```

This distinction is intentional:

- display-only code should not be forced to pretend it has touch
- touch-driven generic code should not be forced to traffic in loose parts if
  it conceptually wants a device

## Example call-site shapes

### Embedded default boot path

```rust
let (mut cyd, calibration_outcome) =
    CydEsp::new(&CYD_STATIC, /* pins... */, &mut flash, &mut button, Some("rebooting")).await?;

if calibration_outcome.was_saved() {
    software_reset();
}

armatron(&mut cyd, &mut button).await?;
```

### Generic app boundary

```rust
async fn run_app(cyd: &mut impl Cyd) -> Result<(), Error> {
    let (display, touch) = cyd.parts();
    // ...
    # let _ = (display, touch);
    Ok(())
}
```

### Test harness path

```rust
let mut cyd = CydMemory::new(size, background, foreground, &FONT);
armatron(&mut cyd, &mut button).await?;
```

## Migration checklist

- [ ] add `Cyd` and `CydBundle` to `device-envoy-core::cyd`
- [ ] add `Cyd` to `device-envoy-core::cyd`
- [ ] implement `Cyd` for `CydEsp`, `CydRp`, `CydMemory`, and wasm CYD
- [ ] replace config-taking `CydEsp::new` / `CydRp::new` with async
      flow-absorbing constructors
- [ ] keep `Cyd*Uncalibrated::new` and display-only constructors public
- [ ] fold `CydError` variants into the esp/rp crate `Error`
- [ ] update core examples/docs to show the new top-level constructor story
- [ ] update esp/rp examples to prefer `CydEsp::new` / `CydRp::new` where they
      want the default boot path
- [ ] move `armatron` and similar generic code back to `C: Cyd`
- [ ] update wasm and any other custom calibration flows to write their
      calibrated touch back into a concrete `Cyd` implementor before entering
      generic runtime code
- [ ] run full device-envoy validation
- [ ] run full linkage-blaze validation

## Non-goals

- restoring the old device-level calibration type-state
- reintroducing `CydScreen`
- making display-only code depend on `Cyd`
- hiding the owned-parts APIs
- introducing a generic `CydBundle<D, T>` as the primary answer to the lost
  bundle abstraction
- adding a platform-independent `Cyd::new`

The owned-parts layer remains the right substrate. This follow-up only
restores the missing bundle abstraction above it.
