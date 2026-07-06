# Remove the `CydIoError` Marker Trait

<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

## Motivation

`CydIoError` (formerly `CydFlushError`) is an empty marker trait in
`device-envoy-core`:

```rust
pub trait CydIoError {}
```

It exists only as the bound on the associated `Error` types of the CYD device
traits. Because it has no methods and no supertraits, generic code bounded by
`S::Error: CydIoError` cannot do anything with the error that it could not do
with an unbounded `type Error`. It also does not help the blanket-`From`
coherence situation documented on `ballet::Error` in
`linkage-blaze-example-core` — that collision exists with or without the
marker.

The only things the marker buys are (a) a single place to attach a future
supertrait such as `Debug` or `defmt::Format`, and (b) a mild opt-in guard
against nonsense `Error` types. Neither has been needed in practice, and the
project convention is to avoid speculative indirection. Delete it; if a shared
requirement on device error types ever materializes, reintroduce a bound with
that requirement at that time.

## Changes in `device-envoy` (this repo)

### `crates/device-envoy-core/src/cyd.rs`

- Delete the `CydIoError` trait definition and its doc comment (currently
  lines 36–44).
- Delete `impl CydIoError for Infallible {}` and its doc comment (currently
  lines 46–47). If nothing else in the file then uses `Infallible`, also
  remove `use core::convert::Infallible;` (doctests import it with their own
  `use` lines and are unaffected).
- `Cyd::Error`: change `type Error: CydIoError;` to `type Error;`.
- `CydDisplay::Error`: change `type Error: CydIoError;` to `type Error;`.
- `CydTouch::Error`: change `type Error: CydIoError;` to `type Error;`.

Keep each associated type's own doc comment ("Error returned when …") — those
describe the role, not the bound.

### `crates/device-envoy-core/src/cyd/touch.rs`

- Remove `use super::CydIoError;`.
- `CydRawTouch::Error`: change `type Error: CydIoError;` to `type Error;`.

Note: `CydFrame::Error` in `crates/device-envoy-core/src/cyd/display.rs` is
already unbounded, so `display.rs` needs no change. It serves as the precedent
that unbounded associated error types are fine on this surface.

### `crates/device-envoy-core/src/memory.rs`

- Remove `CydIoError` from the `use crate::cyd::{…}` import list.
- Delete `impl CydIoError for MemoryCydError {}`.

### `crates/device-envoy-esp/src/cyd.rs`

- Remove `CydIoError` from the `use device_envoy_core::cyd::{…}` import list.
- Delete `impl CydIoError for CydError {}`.

### `crates/device-envoy-rp/src/cyd.rs`

- Remove `CydIoError` from the `use device_envoy_core::cyd::{…}` import list.
- Delete `impl CydIoError for CydError {}`.

### Docs sweep

After the code changes, grep the repo (excluding `target/`) for `CydIoError`
and `CydFlushError` and fix any surviving intra-doc links or prose mentions in
rustdoc comments so `cargo doc` stays link-clean. Historical mentions inside
`specs/` documents may stay as-is.

## Changes in `linkage-blaze` (companion repo)

### `crates/linkage-blaze-example-core/src/ballet.rs`

- The doc comment on `ballet::Error` (currently around line 207) explains the
  blanket-`From` coherence collision in terms of
  `F: CydIoError == device_envoy_core::Error`. Reword it to drop the marker
  reference, e.g. "Rust can't rule out a future `F == device_envoy_core::Error`",
  keeping the rest of the explanation intact. The `.map_err(Error::Flush)`
  pattern itself is unchanged — it never depended on the marker.
- Leave the `todo0000 review this.` comment in place.

### Spec cross-references

`specs/CYD_SURFACE_FOLLOWUPS_HANDOFF_SPEC.md`,
`specs/CYD_SURFACE_FOLLOWUPS_SPEC.md`, and
`specs/CYD_CONTIGUOUS_PIXELS_AND_CORE_ERRORS_SPEC.md` list `CydFlushError` in
their surface inventories. Do not rewrite them wholesale; where a spec is
still active, annotate the mention with a short note that the trait was
renamed to `CydIoError` and then removed by this spec.

## Explicitly out of scope

- No replacement bound (e.g. `Debug`) on the associated `Error` types. Add one
  only when generic code demonstrably needs it.
- No changes to the concrete error types themselves (`MemoryCydError`, the esp
  and rp `CydError` enums) beyond deleting their marker impls.
- No changes to `ballet::Error`'s variants or conversion strategy.

## Verification

1. In `device-envoy`, check and test every crate for its targets the same way
   CI does (core with host/wasm features, esp and rp for their embedded
   targets).
2. In `linkage-blaze`, run `just check-all` (the local CI equivalent).
3. `grep -rn "CydIoError\|CydFlushError"` over both repos' `crates/` trees
   returns no hits.
4. `cargo doc` for `device-envoy-core` builds without broken intra-doc-link
   warnings.
