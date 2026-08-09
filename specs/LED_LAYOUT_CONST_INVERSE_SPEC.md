<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# Store the LED layout inverse at compile time

## Status

Planning only. This specification describes a representation and storage change
for `device_envoy_core::led2d::layout::LedLayout` and the strip-backed RP/ESP
adapters that consume it.

Breaking changes to this unreleased API are allowed. Do not retain compatibility
aliases or duplicate accessor forms during the migration.

## Problem

`LedLayout<N, W, H>` currently stores the physical-wiring-order mapping:

```text
physical_led_index -> (x, y)
```

The runtime renderer needs the inverse mapping:

```text
(x, y) -> physical_led_index
```

Although `LedLayout::xy_to_index` is a `const fn`, the current
`Led2dStripAdapter::new` calls it at runtime and stores the returned
`[u16; N]` by value:

```rust,no_run
pub struct Led2dStripAdapter<'a, const N: usize, S> {
    led_strip: &'a S,
    mapping_by_xy: [u16; N],
    width: usize,
}
```

This performs avoidable work during device construction and retains two bytes
of RAM per LED for the lifetime of the device. A 96-pixel panel uses 192 bytes
for this field; a 256-pixel panel uses 512 bytes.

The layout is already declared as compile-time data. Its inverse should be
derived, checked, and retained with that data rather than recomputed and copied
into each runtime adapter.

## Objective

Make the inverse mapping part of the checked `LedLayout` value, construct both
directions in `LedLayout::new`, and let strip-backed adapters borrow the inverse
table.

The completed change must:

- perform no mapping inversion during RP or ESP device construction;
- store no `[u16; N]` mapping array in `Led2dStripAdapter`;
- preserve the existing physical LED behavior and logical `(x, y)` behavior;
- keep all `LedLayout` constructors and transformations const-evaluable;
- keep `device-envoy-core` `no_std` and allocation-free;
- introduce no `unsafe` code; and
- add no dependency.

## Final `LedLayout` representation

Rename the private `map` field so its direction is explicit, then add the
inverse:

```rust,no_run
pub struct LedLayout<const N: usize, const W: usize, const H: usize> {
    index_to_xy: [(u16, u16); N],
    xy_to_index: [u16; N],
}
```

Both fields remain private. `LedLayout` remains `Clone`, `Copy`, `Debug`,
`PartialEq`, and `Eq` unless implementation experience demonstrates that one of
those derives is no longer valid.

The two arrays express one checked bijection in opposite directions. They must
never be independently supplied by a caller.

## Construct both directions in `new`

`LedLayout::new` remains the validation funnel for explicit mappings,
constructors, and transformations. It accepts only the wiring-order array and
derives the inverse while validating it:

```rust,no_run
pub const fn new(index_to_xy: [(u16, u16); N]) -> Self {
    assert!(W > 0 && H > 0, "W and H must be positive");
    assert!(W * H == N, "W*H must equal N");
    assert!(N <= u16::MAX as usize, "total LEDs must fit in u16");

    let mut seen = [false; N];
    let mut xy_to_index = [0_u16; N];
    let mut led_index = 0;
    while led_index < N {
        let (x, y) = index_to_xy[led_index];
        let x = x as usize;
        let y = y as usize;
        assert!(x < W, "column out of bounds");
        assert!(y < H, "row out of bounds");

        let cell_index = y * W + x;
        assert!(!seen[cell_index], "duplicate (col,row) in mapping");
        seen[cell_index] = true;
        xy_to_index[cell_index] = led_index as u16;
        led_index += 1;
    }

    let mut cell_index = 0;
    while cell_index < N {
        assert!(seen[cell_index], "mapping does not cover every cell");
        cell_index += 1;
    }

    Self {
        index_to_xy,
        xy_to_index,
    }
}
```

Use the repository's current descriptive variable-name conventions in the
implementation. Preserve relevant TODO comments, including the separate TODO
about possibly allowing zero-sized composition identities.

Move the `N <= u16::MAX` assertion from the old inversion path into `new`,
because a successfully constructed layout now always stores every physical
index as `u16`.

Every constructor and transformation must continue to finish through
`LedLayout::new`. Do not bypass validation by constructing the two fields
directly outside `new`.

## Accessors

Keep `index_to_xy` as a borrowed accessor and change `xy_to_index` from a
derived-by-value operation into a borrowed accessor:

```rust,no_run
pub const fn index_to_xy(&self) -> &[(u16, u16); N] {
    &self.index_to_xy
}

pub const fn xy_to_index(&self) -> &[u16; N] {
    &self.xy_to_index
}
```

This signature change is intentional. Update callers directly; do not add a
second copying getter or compatibility method.

Update the method documentation to state both directions explicitly:

```text
index_to_xy[physical_led_index] = (x, y)
xy_to_index[y * W + x] = physical_led_index
```

## Transformations and composition

Continue to implement layout operations from the wiring-order
`index_to_xy` representation:

- `linear_h` and `linear_v`;
- `serpentine_column_major` and `serpentine_row_major`;
- `rotate_cw`, `rotate_ccw`, and `rotate_180`;
- `flip_h` and `flip_v`; and
- `combine_h` and `combine_v`.

Each operation constructs its transformed `index_to_xy` local array and calls
`LedLayout::new`. `new` then derives the corresponding inverse at compile time.
Do not add separate inverse-update logic to every transformation.

Update internal field reads from `self.map` to `self.index_to_xy`. Keep the
current return types, including dimension changes such as:

```text
LedLayout<N, W, H> -> LedLayout<N, H, W>
```

## Runtime adapter storage

Replace the owned mapping field in `Led2dStripAdapter` with a borrowed table:

```rust,no_run
pub struct Led2dStripAdapter<'a, const N: usize, S>
where
    S: LedStrip<N> + ?Sized,
{
    led_strip: &'a S,
    mapping_by_xy: &'static [u16; N],
    width: usize,
}
```

Change its constructor to require the compile-time layout to be available for
the adapter's full lifetime:

```rust,no_run
pub fn new<const W: usize, const H: usize>(
    led_strip: &'a S,
    led_layout: &'static LedLayout<N, W, H>,
) -> Self {
    Self {
        led_strip,
        mapping_by_xy: led_layout.xy_to_index(),
        width: W,
    }
}
```

The exact lifetime spelling may change if Rust requires a separate named
layout lifetime, but the adapter must borrow rather than copy the inverse. Do
not restore an owned `[u16; N]` as a lifetime workaround.

RP and ESP macro-generated layouts are compile-time constants. Verify that the
existing references to those constants receive static promotion at every
generated call site. If relying on promotion is unclear or rejected by the
compiler, make the generated immutable storage explicit without moving the
inverse back into RAM and without duplicating the inversion algorithm outside
`LedLayout::new`.

`Led2dStripBacked::mapping_by_xy` continues to return `&[u16; N]`, and frame
conversion continues to use:

```rust,no_run
let led_index = self.xy_to_index(x_index, y_index);
frame_1d[led_index] = frame_2d[(x_index, y_index)];
```

No frame-conversion or physical output behavior changes are intended.

## Memory tradeoff

This design deliberately trades immutable layout storage for runtime RAM:

- the checked `LedLayout` grows by `2 * N` bytes for its inverse array;
- each strip-backed runtime adapter loses its owned `2 * N` byte array; and
- device construction no longer performs an `O(N)` inversion.

The original wiring-order representation remains stored because it is the
natural source for composition and transforms, is part of the public inspection
API, and is used by documentation tooling. Do not replace it with only the
inverse as part of this change.

Do not claim a flash or RAM improvement solely from Rust type sizes. Confirm in
at least one representative release artifact that the inverse resides in
immutable program data and is not copied into persistent adapter storage.

## Tests

Add focused tests covering the representation and its consumers.

### Layout construction

- Verify a small explicit layout's `index_to_xy` array.
- Verify its exact `xy_to_index` inverse.
- Verify that both accessors work in const contexts.
- Preserve compile-time failures for duplicate coordinates, out-of-bounds
  coordinates, mismatched dimensions, and missing cells.
- Add coverage for the `u16` index limit if practical without creating an
  excessively large compile-time test.

### Constructors and transformations

- Verify both directions after serpentine construction.
- Verify both directions after every rotation and reflection family.
- Verify both directions after `combine_h` and `combine_v`.
- Retain the existing algebra and equality tests.

Tests should prefer small layouts whose complete arrays remain readable.

### Adapter behavior and storage

- Verify that converting a `Frame2d` produces the same `Frame1d` ordering as
  before this change.
- Add a structural size test, using an appropriate fake strip type, proving
  that `size_of::<Led2dStripAdapter<..., N, ...>>()` does not grow linearly with
  `N` because of an owned inverse array.
- Compile representative RP and ESP generated devices using custom composed and
  rotated layouts.
- Ensure multiple devices can borrow the same layout without duplicating its
  inverse in their instances.

Avoid brittle assertions about an exact adapter byte count when an assertion
that compares two `N` values can prove the intended property.

## Documentation

Update `LedLayout` documentation to explain:

- the stored wiring-order direction;
- why drawing requires the inverse direction;
- that `new` proves the relation is one-to-one and constructs both arrays; and
- that runtime adapters borrow the compile-time inverse.

Update doctests and examples affected by the `xy_to_index` return-type change.
Keep one primary compilable `LedLayout` example and link method documentation
to it according to repository conventions.

The separate `rust-const-fn` article draft is outside this implementation
specification. Update it only as a separately requested editorial task after
the API has settled.

## Rejected alternatives

### Keep computing the inverse in `Led2dStripAdapter::new`

Rejected because it performs avoidable runtime work and permanently consumes
RAM proportional to the LED count.

### Generate a separate inverse constant in each platform macro

Rejected because it splits one checked bijection across the layout abstraction
and platform-specific macro expansions. `LedLayout::new` is the canonical place
to establish both directions.

### Store only `xy_to_index`

Rejected for this change because wiring-order data is the natural source for
layout construction, composition, transformations, inspection, and the SVG
documentation renderer.

### Return the inverse by value for compatibility

Rejected because it retains an easy accidental-copy path and creates two ways
to access the same data. Migrate callers to the borrowed accessor.

## Validation

Run, at minimum:

```text
cargo fmt --all -- --check
cargo test -p device-envoy-core --features host
cargo check-all
```

The full `cargo check-all` run must continue to build every supported embedded
target. Do not skip targets because of a missing toolchain component.

Inspect a representative optimized RP or ESP artifact or map output to confirm
that:

- the inverse table is emitted as immutable data;
- the runtime adapter contains only a reference to it; and
- no duplicate persistent `[u16; N]` copy remains in RAM.

## Acceptance criteria

The work is complete when:

- `LedLayout` owns checked `index_to_xy` and `xy_to_index` arrays;
- `LedLayout::new` constructs and validates both in const evaluation;
- `xy_to_index` returns a borrowed fixed-size array;
- all constructors, transformations, and compositions still pass through
  `LedLayout::new`;
- `Led2dStripAdapter` borrows the inverse and stores no owned mapping array;
- logical drawing and physical LED ordering are unchanged;
- focused construction, transformation, inversion, storage, RP, and ESP tests
  pass;
- a representative optimized artifact confirms the intended immutable-storage
  and RAM behavior; and
- `cargo check-all` passes.
