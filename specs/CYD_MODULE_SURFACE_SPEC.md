# Spec: Trim the `cyd` module's public surface and rename `RegionPixels`

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

## Goal

The rustdoc index for `device_envoy_core::cyd` currently mixes the app-facing
API with calibration/implementor plumbing, because the whole `calibration`
module is flattened into `cyd` via re-exports. Restructure so the `cyd` index
shows only what app authors use, while implementor-facing items remain public
one level down in a `cyd::calibration` submodule. Also rename the
`RegionPixels` trait to `RectanglePixels`.

Per workspace rules: express visibility through actual visibility modifiers
(never `#[doc(hidden)]`), and do **not** leave backwards-compatibility aliases
or shims — refactor call sites directly.

## Repos and crates affected

- `device-envoy` (primary): `device-envoy-core`, `device-envoy-esp`,
  `device-envoy-rp`, plus example files and the `.j2` example templates.
- `linkage-blaze` (downstream, sibling checkout at
  `~/programs/linkage-blaze`): `linkage-blaze-classic` (the `armatron`
  example) and `linkage-blaze-armatron-wasm`.

## Change 1 — Rename `RegionPixels` to `RectanglePixels`

`RegionPixels` (`crates/device-envoy-core/src/cyd.rs`, ~line 49) is a borrowed
or owned rectangular RGB565 pixel region; `RectanglePixels` matches the
`embedded_graphics::primitives::Rectangle` vocabulary used throughout the API.

- Rename the trait and update its doc comment if it still says "region".
- Update every implementor and use site. Known locations (re-grep to be sure):
  - `device-envoy-core/src/cyd.rs` (`CydDisplay::flush_at` signature)
  - `device-envoy-core/src/memory.rs` (`MemoryFrame` impl and tests)
  - `device-envoy-esp/src/cyd.rs`, `src/cyd/display.rs`, `src/cyd/buffer.rs`
  - `device-envoy-rp/src/cyd.rs`, `src/cyd/display.rs`, `src/cyd/buffer.rs`
- Note the existing `RegionView` type in the esp/rp `buffer.rs` files: it
  implements this trait. Renaming `RegionView` itself is **out of scope**
  unless it falls out trivially; if left as-is, do not add a comment
  apologizing for it.
- There is a `TODO0x Arg! "region" (may no longer apply)` comment near
  `CydFrame::rectangle` in `cyd.rs`. Do not delete it; if this rename
  resolves part of it, append `(may no longer apply)` context rather than
  removing it (workspace TODO policy).

`RectanglePixels` stays at the `cyd` module level: it is the parameter type of
the public provided method `CydDisplay::flush_at`, so it is app-visible API.

## Change 2 — Make `calibration` a public submodule; trim the flat re-exports

In `crates/device-envoy-core/src/cyd.rs`:

1. Change `pub(crate) mod calibration;` to `pub mod calibration;`.
2. Replace the current eight-item flat re-export

   ```rust
   pub use calibration::{
       CalibrationConfig, EnsureCalibrationError, EnsureCalibrationOutcome,
       EnsureCalibrationSettings, RawPoint, RawTouchEvent, ensure_calibration,
       ensure_calibration_with_settings,
   };
   ```

   with only the items every board app touches:

   ```rust
   pub use calibration::{EnsureCalibrationError, EnsureCalibrationOutcome, ensure_calibration};
   ```

   (`EnsureCalibrationOutcome` stays re-exported because it is the return
   type of `ensure_calibration`.)

3. The remaining items stay public but are now reached through the
   submodule: `cyd::calibration::{CalibrationConfig, EnsureCalibrationSettings,
   RawPoint, RawTouchEvent, ensure_calibration_with_settings}`.

4. Move the `CydRawTouch` trait (currently defined in `cyd.rs`, ~line 378)
   into `cyd/calibration.rs`. Its own doc comment says it "exists
   specifically for the shared calibration driver", so that is where it
   belongs. Do **not** re-export it at the `cyd` level.

5. Give `cyd/calibration.rs` a short module doc comment (if it lacks one)
   explaining it is the touch-calibration driver plus the raw-touch plumbing
   that device implementations provide, linking to `ensure_calibration` as
   the primary entry point (workspace docs convention: link readers to the
   primary item).

### Import fallout (update, do not alias)

Re-grep for each moved/renamed path; known users:

- `device-envoy-core/src/memory.rs` — implements `CydRawTouch`, constructs
  `RawTouchEvent`/`RawPoint`; tests use `CalibrationConfig`,
  `EnsureCalibrationError`, `EnsureCalibrationOutcome`, `RawPoint`,
  `RawTouchEvent`, `ensure_calibration`.
- `device-envoy-core/src/wasm.rs` — implements `CydRawTouch`, uses
  `CalibrationConfig`.
- `device-envoy-core/src/cyd/calibration/driver.rs` and `flow.rs` — internal
  paths and doc examples reference `device_envoy_core::cyd::{...}` paths;
  update the doctest imports to the new `cyd::calibration::` paths.
- `device-envoy-esp/src/cyd.rs`, `src/cyd/touch.rs`; `device-envoy-rp/src/cyd.rs`,
  `src/cyd/touch.rs`, `src/error.rs` — implement `CydRawTouch`, construct
  raw events, name `EnsureCalibrationError` in error plumbing.
- Board examples using `ensure_calibration` keep working unchanged
  (it remains at `cyd` level), but check the `cyd_touch_paint` examples in
  `device-envoy-esp/examples/**` and `device-envoy-rp/examples/` anyway.
- **Templates**: `device-envoy-esp/examples/templates/cyd_touch_paint.rs.j2`
  (and any sibling `.j2` templates) must be updated in lockstep with the
  generated examples, or regeneration will reintroduce old paths.
- Doc examples on the `Cyd` trait in `cyd.rs` reference
  `device_envoy_core::cyd::CopySizeError` etc. — verify all doctest paths
  still resolve.

Downstream in `linkage-blaze`:

- `crates/linkage-blaze-classic/examples/armatron.rs` — uses
  `EnsureCalibrationError`, `ensure_calibration` (both still at `cyd` level;
  should compile unchanged, but verify).
- `crates/linkage-blaze-armatron-wasm/src/lib.rs` — uses `CalibrationConfig`,
  `EnsureCalibrationSettings`, `ensure_calibration_with_settings`; update the
  imports to `device_envoy_core::cyd::calibration::{...}`.

## Change 3 — State the literal values in the screen-constant docs

The docs for `SCREEN_WIDTH`, `SCREEN_HEIGHT`, and `SCREEN_PIXELS` in
`crates/device-envoy-core/src/cyd.rs` describe the constants but never state
their values, so the module index (which shows only the first doc line) tells
the reader nothing concrete. Include the literal value in each doc comment's
first line, e.g.:

```rust
/// Native panel width in pixels (landscape): 320. The CYD panel is fixed hardware.
pub const SCREEN_WIDTH: usize = 320;
/// Native panel height in pixels (landscape): 240. The CYD panel is fixed hardware.
pub const SCREEN_HEIGHT: usize = 240;
/// Total panel pixel count (`SCREEN_WIDTH * SCREEN_HEIGHT` = 76,800).
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;
```

Exact wording may be adjusted, but each first line must contain the literal
number (320, 240, 76,800).

## Explicitly out of scope

- Demoting `RectanglePixels`, `CydFlushError`, `CydInfallibleError`,
  `CopySizeError`, `ContiguousPixels`, or `Tiles` — all are load-bearing
  app-facing API (trait bounds, return types, or wrapped error types in
  downstream error enums).
- Rethinking whether `CydDisplay::flush_at` belongs on the app-facing trait.
- Renaming `RegionView` in the esp/rp buffer modules (see Change 1).

## Verification

1. In `device-envoy`: run `just check-all` (local CI — tests, checks, and
   builds all crates across all targets). This must pass, including doctests.
2. In `linkage-blaze`: run `just check-all` against the sibling checkout to
   confirm the downstream crates and examples still build.
3. Build the docs (`cargo doc -p device-envoy-core`) and confirm the
   `cyd` module index now lists: the five `Cyd*` app traits (`Cyd`,
   `CydDisplay`, `CydTouch`, `CydFrame`, plus marker `CydFlushError`),
   `RectanglePixels`, `Tiles`, `ContiguousPixels`, `CopySizeError`,
   `CydInfallibleError`, `DrawItem`, `Image565*`, `Orientation`,
   `TouchEvent`, the `tga565*` macros, the `SCREEN_*` constants, the
   `tiling` and `calibration` submodules, and only the three calibration
   re-exports — with `RawPoint`, `RawTouchEvent`, `CalibrationConfig`,
   `EnsureCalibrationSettings`, `ensure_calibration_with_settings`, and
   `CydRawTouch` visible only under `cyd::calibration`.

Suggested commit message:

```text
Trim cyd module surface: public calibration submodule, move CydRawTouch into it, rename RegionPixels to RectanglePixels
```
