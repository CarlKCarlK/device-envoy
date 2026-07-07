# CYD Doc-Review Follow-Ups

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

Concerns raised while reading the generated rustdoc for
`device_envoy_core::cyd` and `trait.Cyd` after the bundle follow-ups
(`CYD_BUNDLE_FOLLOWUPS_SPEC.md`) landed. Items are being collected one at a
time as the review proceeds.

## 1. Rename `UnwrapNever` → `UnwrapInfallible`

`device_envoy_core::UnwrapNever` with `.unwrap_never()` reads wrong; the
error type it targets is `core::convert::Infallible`, so the name should say
so:

```rust
pub trait UnwrapInfallible {
    type Output;
    fn unwrap_infallible(self) -> Self::Output;
}

impl<T> UnwrapInfallible for core::result::Result<T, Infallible> { ... }
```

Scope of the rename:

- [ ] core `error.rs`: rename trait and method; update the `lib.rs` re-export
- [ ] core call sites (`memory.rs`, doc examples)
- [ ] linkage-blaze `AGENTS.md`: the workspace rule that says "use
      `.unwrap_never()` (from the local infallible-result extension)" —
      update to `.unwrap_infallible()`
- [ ] linkage-blaze: delete the now-duplicate local
      `linkage-blaze-example-core/src/infallible.rs`
      (`InfallibleResultExt::unwrap_infallible`) and switch its users
      (e.g. `skeleton_clock.rs`) to the shared
      `device_envoy_core::UnwrapInfallible` — one source of truth, same
      method name, so those call sites only change their `use` line

No backwards-compatibility shims — rename directly (per AGENTS.md).

## 2. `CydTouch` / `CydTouchUncalibrated` belong at the `cyd` module level

They currently live one level down, in `cyd::touch`, while their sibling
`CydDisplay` is defined directly in `cyd.rs`. Move both trait definitions
into `cyd.rs` (near `Cyd` and `CydDisplay`), alongside the imports and
call-site fixes that follow from the move:

- [ ] core `cyd.rs`: add `CydTouch` / `CydTouchUncalibrated` trait
      definitions (moved out of `cyd/touch.rs`); update the module's
      top `//!` doc, which currently links `[`touch::CydTouch`]` /
      `[`touch::CydTouchUncalibrated`]`, to link the module-level traits
      directly
- [ ] core `cyd/touch.rs`: remove the trait definitions; its module doc
      comment referencing them needs `super::` qualification
- [ ] core `cyd/touch/driver.rs`: import site (`use super::CydTouchUncalibrated`
      → `use super::super::{CydDisplay, CydTouchUncalibrated}`) and the
      doctest's `cyd::{..., touch::{CydTouch, CydTouchUncalibrated, ...}}`
      import
- [ ] core `memory.rs`, `wasm.rs`: `use crate::cyd::touch::{CydTouch,
      CydTouchUncalibrated, ...}` → split, with the two traits imported from
      `crate::cyd` directly
- [ ] core `memory.rs` test module (`mod tests`): same import split — still
      pending as of this writing (`cargo test --features host --lib --tests`
      fails on this one path)
- [ ] confirm no other crate (esp, rp, linkage-blaze) references the old
      `cyd::touch::CydTouch` / `cyd::touch::CydTouchUncalibrated` paths
      (checked during this review: none do — only the module doc-comment
      link above)

## 3. Restore the `Cyd`-trait-level doctest and module pointer

Before the type-state/owned-parts refactors, `cyd.rs`'s module doc comment
said plainly:

> See [`Cyd`] for the primary trait and usage example.

and the **full runnable doctest lived on `Cyd`'s own doc comment** — not on
`CydDisplay`. In the current tree the example migrated onto `CydDisplay`
(`#[cfg_attr(feature = "host", doc = r#"..."#)]` block), and the module doc
instead says "See the [module documentation](self) for a simple end-to-end
example," pointing at itself rather than at a specific trait. Two things
drifted from the old convention:

- The module-level `//!` doc no longer names `Cyd` as the place to look for
  the usage example.
- The runnable example itself sits on `CydDisplay`, not on `Cyd` — even
  though `Cyd` is now the primary bundle type a reader constructs first and
  the one the module doc introduces first.

Fix: move the `#[cfg_attr(feature = "host", doc = ...)]` runnable-example
block from `CydDisplay` onto `Cyd`, and restore a direct module-doc pointer
to it, e.g. "See [`Cyd`] for the primary trait and usage example." The
example content itself (construct a `CydMemory`, call `.parts()` or use the
bundle directly, write text, read touch, flush) does not need to change —
only which item's doc comment hosts it, and the module-doc cross-reference.

- [ ] core `cyd.rs`: move the doctest from `CydDisplay` to `Cyd`
- [ ] core `cyd.rs`: update the module `//!` doc to point at `Cyd` for the
      example, matching the pre-refactor wording
- [ ] core `cyd.rs`: audit other doc comments that say "See the [module
      documentation](self)" or "See the [CydDisplay trait
      documentation](Self)" for a usage example — repoint the ones that
      meant the `Cyd` example specifically

## 4. `Cyd`-mediated `decalibrate()` is not actually reachable

`CYD_OWNED_PARTS_SPEC.md`'s recalibration doctrine describes a wasm-only
"owned round trip": call `touch.decalibrate()`, rerun `ensure_calibration`,
get a fresh calibrated touch back. That is not possible through the `Cyd`
trait as implemented:

```rust
pub trait Cyd {
    fn parts(&mut self) -> (&mut Self::Display, &mut Self::Touch);
    fn touch(&mut self) -> &mut Self::Touch { self.parts().1 }
    // ...
}
pub trait CydTouch: Sized {
    fn decalibrate(self) -> Self::Uncalibrated;   // consumes `self` by value
}
```

`Cyd::touch()` only ever hands back `&mut Self::Touch`. `decalibrate` takes
`self` by value and returns a *different type* (`Self::Uncalibrated`), so it
cannot be called through a `&mut` borrow — the same root constraint that
motivated owned parts in the first place: a `&mut T` can never become a
`&mut U`.

Checked every real call site of `.decalibrate()` in the tree; none of them
go through `Cyd`:

- `CydWasm::parts_uncalibrated()` / `CydMemory::parts_uncalibrated()` clone
  the stored field first, so they own a fresh value before decalibrating it.
- One `memory.rs` test decalibrates a `touch` returned *by value* from
  `ensure_calibration`, never obtained via `.parts()`.

And the one app that actually needs in-process recalibration —
`linkage-blaze-armatron-wasm` — never calls `.decalibrate()` either: its
loop sidesteps the problem entirely by constructing a brand-new
`CydWasm::new(...)` on every iteration instead of mutating one in place.
That's the tell that the documented doctrine doesn't match what was actually
buildable.

Options to reconcile the spec with reality:

- **(a) Narrow the doctrine to what's true.** Recalibration-in-place is a
  concrete-type-level operation (own the whole bundle by value, destructure
  it, decalibrate the touch field, rebuild), not something `&mut impl Cyd`
  can do generically. Document `Cyd` as read/write access to already-
  calibrated parts only; drop the implied generic decalibrate-through-`Cyd`
  round trip from the spec text, and either keep the "rebuild a fresh
  device" pattern as the endorsed wasm answer (matching what
  armatron-wasm already does) or drop `decalibrate()`/`Uncalibrated` from
  `Cyd`'s useful surface notes.
- **(b) Give `Cyd` an owned exit.** Add something like
  `fn into_parts(self) -> (Self::Display, Self::Touch)` (consuming `self`)
  alongside the `&mut` `parts()`. Lets code that owns a `C: Cyd` by value
  decalibrate its touch — but then rebuilding a `C` generically needs a
  `From<(Self::Display, Self::Uncalibrated)>`-style bound or similar, adding
  back some of the complexity owned-parts was meant to avoid.
- **(c) Leave `Cyd` `&mut`-only and accept recalibration is always a
  concrete-type operation**, same conclusion as (a) but framed as "this was
  never a `Cyd` capability to begin with" rather than something to patch.

**Resolved:** option (b), extended with a `Cyd::from_parts` constructor so
the round trip closes generically. Specified in
`CYD_INTO_FROM_PARTS_SPEC.md`, including a `CydMemory` round-trip test
(decalibrate → `ensure_calibration` → `from_parts`) and the deletion of the
`CydWasm::set_calibration_config` workaround.

<!-- Further doc-review items will be added below as the review continues. -->
