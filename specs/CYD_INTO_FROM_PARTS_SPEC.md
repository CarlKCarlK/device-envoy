# CYD `Cyd::into_parts` / `Cyd::from_parts`: an Owned Exit and Entry

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Resolves item 4 of `CYD_DOC_REVIEW_SPEC.md` (decision: option 2). The `Cyd`
trait currently only lends its halves (`parts(&mut self)` returns `&mut`
borrows), so a calibrated touch can never be `decalibrate()`d through a
`Cyd` — that method consumes `self` by value and returns a different type.
This spec gives `Cyd` an **owned exit and entry**, making the
decalibrate → recalibrate → rebuild round trip expressible for any `C: Cyd`.

## Trait change

```rust
pub trait Cyd: Sized {
    type Error;
    type Display: CydDisplay<Error = Self::Error>;
    type Touch: CydTouch<Error = Self::Error>;

    /// Borrow both calibrated halves at once.
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);

    /// Consume the device into its owned halves.
    ///
    /// This is the exit that makes ownership-level touch transitions
    /// (`CydTouch::decalibrate`) reachable from a `Cyd`.
    fn into_parts(self) -> (Self::Display, Self::Touch);

    /// Reassemble a device from its owned halves.
    ///
    /// The halves must originate from the same underlying device: for
    /// hardware bundles that is enforced naturally (there is only one of
    /// each), but for `Rc`-backed harnesses (`CydMemory`, `CydWasm`) pairing
    /// parts from two different harnesses is not detected and yields a
    /// device whose halves observe different shared state.
    fn from_parts(display: Self::Display, touch: Self::Touch) -> Self;

    // display() / touch() default accessors unchanged
}
```

`Sized` becomes a supertrait bound (required by the by-value methods; all
implementors are concrete structs and no `dyn Cyd` exists).

## Implementations

All four implementors are field pairs, so both methods are one-liners:

- **`CydEsp` / `CydRp`**: destructure / construct the two `pub` fields.
- **`CydWasm`**: same (its two fields are cheap `Rc`-handle clones).
- **`CydMemory`**: `into_parts` drops the harness's own `shared` handle and
  yields the two parts (each carries its own `Rc` to the shared state);
  `from_parts` reconstitutes the harness's `shared` handle from the display
  part's `Rc` (internal field access), so the inspection surface
  (`pixel()`, `flush_count()`, `push_touch_event()`, scripting) keeps
  working after a round trip.

## Consequence: delete `CydWasm::set_calibration_config`

The wasm armatron loop currently works around the missing owned entry with a
mutator: it runs the flow at parts level, then copies the resulting config
into the harness's stored touch via `set_calibration_config`. With
`from_parts` that smell disappears:

```rust
// linkage-blaze-armatron-wasm, per loop iteration (replaces the mutator):
let source_cyd = CydWasm::new(context.clone(), ORIENTATION, BACKGROUND, FOREGROUND, &FONT, touch_source.clone());
let (mut display, touch_uncalibrated) = source_cyd.parts_uncalibrated();
let (touch, calibration_outcome) = ensure_calibration_with_settings(
    &mut display, touch_uncalibrated, &mut flash, &mut button, None, settings,
).await?; // (existing error handling unchanged)
let mut cyd = CydWasm::from_parts(display, touch);
armatron(&mut cyd, &mut button).await
```

Delete `CydWasm::set_calibration_config` — `from_parts` is the supported way
to install a flow result into a bundle.

## Doctrine update (docs)

The recalibration story in the `cyd` module / `Cyd` docs becomes:

- **ESP / RP (hardware)**: clear flash + software reset stays the endorsed
  path — the boot flow re-runs `ensure_calibration`.
- **In-process (wasm, or any owner-by-value)**: now a real, generic round
  trip — `into_parts` → `decalibrate` → `ensure_calibration` →
  `from_parts`.

## Coverage: memory round-trip test

Add to `memory.rs`'s `mod tests` (imports for `ensure_calibration`,
`EnsureCalibrationOutcome`, `FlashBlockMemory`, `block_on`, and the existing
`test_cyd_memory()` helper are already present; add `Cyd` and `CydTouch`
trait imports as needed):

```rust
#[test]
fn cyd_into_from_parts_decalibrates_and_recalibrates() {
    let cyd = test_cyd_memory();
    let saved_config = CalibrationConfig::new(1.0, 0.0, 2.0, 0.0, 1.0, 3.0);
    let mut memory_flash_block = FlashBlockMemory::with_value(&saved_config);
    let mut memory_button = cyd.button_memory();

    // Owned exit from the bundle, then down to a raw touch.
    let (mut display, touch) = cyd.into_parts();
    let touch_uncalibrated = touch.decalibrate();

    // Recalibrate through the shared flow. The flash already holds a
    // config, so the flow returns `Loaded` without any tap scripting.
    let (touch, outcome) = block_on(ensure_calibration(
        &mut display,
        touch_uncalibrated,
        &mut memory_flash_block,
        &mut memory_button,
        None,
    ))
    .expect("preloaded calibration should load");
    assert!(matches!(outcome, EnsureCalibrationOutcome::Loaded(_)));
    assert_eq!(touch.calibration_config(), saved_config);

    // Owned entry back into a whole device; it must be fully usable and the
    // harness inspection surface must still observe it.
    let mut cyd = CydMemory::from_parts(display, touch);
    cyd.push_touch_event(TouchEvent::Down {
        point: Point::new(12, 34),
    });
    {
        let (display, touch) = cyd.parts();
        assert!(matches!(
            touch.read().expect("touch read should succeed"),
            Some(TouchEvent::Down { .. })
        ));
        let mut frame = display.full_frame_mut();
        block_on(frame.flush()).expect("flush should succeed");
    }
    assert_eq!(cyd.flush_count(), 1);
}
```

The test exercises, in order: `Cyd::into_parts`, `CydTouch::decalibrate`,
the flow re-entry, `Cyd::from_parts`, and that the reassembled harness still
reads touch, flushes frames, and reports `flush_count()` — i.e. the shared
state survived the round trip.

## Migration checklist

- [ ] core `cyd.rs`: add `Sized` supertrait, `into_parts`, `from_parts`
      (with the same-origin doc note); update the recalibration doctrine in
      the module/`Cyd` docs
- [ ] core `memory.rs`: implement both methods on `CydMemory`; add the
      round-trip test above
- [ ] core `wasm.rs`: implement both methods on `CydWasm`; delete
      `set_calibration_config`
- [ ] esp / rp: implement both methods on `CydEsp` / `CydRp`
- [ ] linkage-blaze `armatron-wasm`: replace the `set_calibration_config`
      call with `CydWasm::from_parts` per the snippet above
- [ ] `cargo check-all` (device-envoy) and `just check-all` (linkage-blaze)
      both green

No backwards-compatibility shims — refactor call sites directly (per
AGENTS.md).
