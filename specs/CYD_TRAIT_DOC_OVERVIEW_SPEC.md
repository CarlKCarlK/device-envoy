# CYD Trait Doc Overview & Memory Rename Spec

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Improve the `device_envoy_core::cyd` docs so the overview and the primary
example live together, once, on the `Cyd` trait page; make the example a real
runnable doctest against the in-memory device; and fix naming that is
inconsistent with the platform crates.

## 1. Rename the `memory` module types (Cyd-first convention)

The platform crates put the device family first and the platform qualifier
last:

- `device-envoy-esp`: `CydEsp`, `CydStaticEsp`, `CalibratedCydEsp`, `CydDisplayEspPart`, `CydTouchEspPart`, `CydFrameEsp`
- `device-envoy-rp`: `CydRp`, `CydStaticRp`, `CalibratedCydRp`, `CydDisplayRpPart`, `CydTouchRpPart`, `CydFrameRp`
- `device-envoy-core::wasm`: `CydWasm`, `CydDisplayWasmPart`, `CydTouchWasmPart`, `ButtonWasm`, `ButtonWasmSource`

The `memory` module is the odd one out — it puts `Memory` first. Rename:

| Current | New |
| --- | --- |
| `MemoryCyd` | `CydMemory` |
| `MemoryCydError` | `CydMemoryError` |
| `MemoryDisplayPart` | `CydDisplayMemoryPart` |
| `MemoryTouchPart` | `CydTouchMemoryPart` |
| `MemoryFrame` | `CydFrameMemory` |
| `MemoryButton` | `ButtonMemory` |
| `MemoryFrameClock` (pub(crate)) | `FrameClockMemory` |
| `MemoryCyd::memory_button()` | `CydMemory::button_memory()` (variable-matches-type naming) |
| `MemoryFlashBlock` / `MemoryFlashDevice` (test-only) | `FlashBlockMemory` / `FlashDeviceMemory` |

The `wasm` module already follows the convention — no changes there.

Per AGENTS.md, no backwards-compatibility aliases: rename aggressively and fix
all call sites. Known usage sites to update:

- `device-envoy-core`: `memory.rs`, `cyd.rs` (the `host` `cfg_attr` doc line),
  `button.rs`, and any tests.
- Downstream **linkage-blaze** (separate repo, must be updated in the same
  wave): `crates/linkage-blaze-example-core/src/{clock.rs, ballet.rs,
  skeleton_clock.rs, armatron/main.rs}` import
  `device_envoy_core::memory::MemoryCyd`.
- Doc prose that says "MemoryCyd" (e.g. the memory module doc, the
  `assert_framebuffer_matches_expected_png` docs).

## 2. Add `unwrap_never()` to `device-envoy-core`

The improved example draws with `embedded_graphics`, whose `DrawTarget` impl on
`CydFrameMemory` has `Error = Infallible`. Repo policy forbids `.unwrap()` /
`.expect()` / `let _ =` on `Result`s and prescribes `.unwrap_never()` for
`Infallible` errors — but no such extension exists in device-envoy yet. Add it
to `device-envoy-core` (suggested home: `src/error.rs`, re-exported from the
crate root alongside `Result`):

```rust
use core::convert::Infallible;

/// Extension for unwrapping a `Result` whose error type is uninhabited.
pub trait UnwrapNever {
    type Output;

    /// Unwrap a `Result<T, Infallible>` without a possible panic path.
    fn unwrap_never(self) -> Self::Output;
}

impl<T> UnwrapNever for Result<T, Infallible> {
    type Output = T;

    fn unwrap_never(self) -> T {
        match self {
            Ok(value) => value,
            // No Err arm: `Infallible` is uninhabited.
        }
    }
}
```

(If the compiler requires it, `Err(never) => match never {}` is the fallback
arm; on edition 2024 the uninhabited variant should be allowed to be omitted.)

## 3. Consolidate the overview onto the `Cyd` trait page

Today the prose is split: `cyd.rs` module docs carry the hardware overview
(ILI9341 + XPT2046, the parts bullet list) and point at `Cyd`; the `Cyd` trait
carries the example. Per the AGENTS.md convention ("link readers to the primary
type and keep a single compilable example on that type"), move the overview so
overview + example appear once, together, on `trait.Cyd.html`:

- **`Cyd` trait doc** gains the overview paragraphs currently at the top of
  `cyd.rs`: what a CYD board is, that `Cyd` is the whole device, that
  [`Cyd::parts`] borrows the [`CydDisplay`] and [`CydTouch`] halves, and that
  the [`display`] / [`touch`] submodules hold support types. The example
  (section 4) follows directly.
- **`cyd` module doc** shrinks to one or two sentences: name the module, link
  to the `Cyd` trait as the primary type ("See the [`Cyd`] trait for the
  overview and a usage example."). Delete the duplicated bullets and prose.
- Keep the existing `cfg_attr` doc lines on `Cyd` pointing at
  `CydMemory` (renamed) and `CydWasm`.

## 4. Replace the example with a runnable memory-backed doctest

**Yes — this can be a genuinely runnable doctest** using the in-memory device,
which also deletes the ~90 hidden lines of `DemoCyd`/`DemoDisplay`/`DemoFrame`
boilerplate currently propping up the `no_run` example. Precedent: the
`memory` module doc already has a runnable doctest driving `parts()` +
`flush()` via `futures_executor::block_on` (a dev-dependency).

Shape of the new example on `Cyd` (final code to be tuned until
`just check-docs-core` passes):

```rust
use device_envoy_core::UnwrapNever;
use device_envoy_core::cyd::{Cyd as _, CydDisplay, CydTouch, display::CydFrame, touch::TouchEvent};
use device_envoy_core::memory::CydMemory;
use embedded_graphics::{
    Drawable,
    mono_font::ascii::FONT_9X15_BOLD,
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Point, RgbColor, Size},
    primitives::{Circle, Primitive, PrimitiveStyle},
};

# futures_executor::block_on(async {
# let mut cyd = CydMemory::new(Size::new(320, 240), Rgb888::BLACK, Rgb888::WHITE, &FONT_9X15_BOLD);
# cyd.push_touch_event(TouchEvent::Down { point: Point::new(160, 120) });
let (mut display, mut touch) = cyd.parts();
let mut frame = display.full_frame_mut();
frame.write_text("Hello CYD");

// An app would usually run this in a loop: read touch, draw, flush, repeat.
if let Some(TouchEvent::Down { point } | TouchEvent::Move { point }) = touch.read()? {
    Circle::with_center(point, 24)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(&mut frame)
        .unwrap_never();
}
frame.flush().await?;
# assert_eq!(cyd.pixel(160, 120), Rgb565::RED);
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
# })?;
# Ok::<(), device_envoy_core::memory::CydMemoryError>(())
```

Notes and decisions:

- **Runnable, not `no_run`.** AGENTS.md prefers `rust,no_run`, but here a
  plain `rust` fence is deliberate: the memory device makes the example a real
  test (the hidden `assert_eq!` on the touched pixel proves the circle drew).
  This matches the existing runnable doctest in the `memory` module. Call this
  out in the commit message.
- **Hidden lines** (`#`-prefixed): the `block_on(async { ... })` wrapper (so
  the visible code reads as natural `async` device code with `.await?`), the
  `CydMemory` construction, the scripted `push_touch_event`, and the final
  pixel assertion. Visible code is exactly the device-facing flow.
- **Single flush per iteration**: text and the touch-response circle are drawn
  into the same frame and flushed once, matching how a real per-frame loop
  works. (The earlier sketch flushed the text before reading touch; folding it
  into one flush reads better as a loop body.)
- **Feature gating**: the example depends on `CydMemory` (`host` feature).
  Docs and doctests always run with `--features host,wasm`
  (`just check-docs-core`), but to keep a bare `cargo test --doc` from
  breaking, attach the example via the existing pattern on `Cyd`:
  `#[cfg_attr(feature = "host", doc = "...")]` (multi-line doc string), placed
  after the always-present overview prose. Without `host` the trait page shows
  the overview and the pointer lines only.
- **Touch pattern**: `TouchEvent::Down { point } | TouchEvent::Move { point }`
  shows both x-y-carrying variants; `TouchEvent::Up` falls through.
- Update the smaller "See the [Cyd trait documentation](Cyd) for a usage
  example" cross-references only if the example's location or meaning changes
  (it stays on `Cyd`, so they should all remain valid).

## 5. Verification

- `just check-all` at the device-envoy repo root (includes
  `cargo test --doc --features host,wasm` and the rustdoc-warnings-as-errors
  build).
- Build docs (`just show-docs-core`) and confirm `trait.Cyd.html` shows
  overview + example together and `cyd/index.html` is a short pointer.
- Rebuild linkage-blaze (`just check-all` there) after the renames land, since
  its example-core tests import the memory device.

## Suggested implementation order

1. Renames (section 1) — mechanical, done first so docs/example are written
   against final names.
2. `unwrap_never()` (section 2).
3. Doc consolidation + new example (sections 3–4).
4. Downstream linkage-blaze fixups, then verification (section 5).
