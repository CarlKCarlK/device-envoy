<!-- TODO0 Consider deleting this spec once the DrawItem rustdoc cleanup is implemented and released. -->

# DrawItem Rustdoc Cleanup

## Goal

Make the `DrawItem` rustdoc page contain one clear, executable visual example
that teaches how to construct and draw a bitmap.

`DrawItem` remains in `device-envoy-core`. Device Envoy implements
`DrawItem::draw`, uses `DrawItem` in its touch-calibration and contiguous-drawing
paths, and does not depend on linkage-blaze. Linkage-blaze is only one producer
of projected `DrawItem` values.

## Required change

Edit `crates/device-envoy-core/src/cyd/display/draw_item.rs`:

1. Delete the old `rust,no_run` `DrawItem` example that constructs `Stroke`,
   `Ellipse`, `Circle`, and `Bitmap` in four dense lines.
2. Keep the `CydMemory` bitmap example as the one canonical `DrawItem` example.
3. Keep that example focused on this sequence:
   - load the existing `docs/assets/cyd_fill_contiguous.tga` demo bitmap at
     compile time;
   - construct `CydMemory` and a frame;
   - construct `DrawItem::Bitmap` with `Image565View`;
   - construct at least one additional shape, such as `DrawItem::Circle`;
   - call `DrawItem::draw` for both items;
   - flush the frame;
   - compare it with `draw_item_bitmap.png` using
     `assert_framebuffer_matches_expected_png`;
   - display the embedded PNG when `doc-images` is enabled.
4. Hide setup and golden-image assertion lines with rustdoc `#` prefixes when
   they do not help explain `DrawItem`. Keep the visible code understandable as
   one example from top to bottom.
5. Do not use `unwrap`, `expect`, `.ok()`, or `let _ =` in the doctest. Follow
   the repository error-handling rules and preserve useful errors where
   practical.
6. Ensure the introductory prose describes the single example accurately. Do
   not refer to a `no_run` example as compiled or imply another example follows.

Edit `crates/device-envoy-core/src/pixel_target.rs` only if needed:

- Keep `PixelTarget` documentation concise and link to the canonical
  `DrawItem` bitmap example. Do not duplicate the example there.

Keep these generated golden assets synchronized:

- `crates/device-envoy-core/tests/assets/draw_item_bitmap.png`
- `crates/device-envoy-core/docs/assets/draw_item_bitmap.png`

Do not change the `DrawItem` API, rendering behavior, variant set, or module
location as part of this cleanup.

## Verification

Run:

```text
rustfmt --edition 2024 --check crates/device-envoy-core/src/cyd/display/draw_item.rs crates/device-envoy-core/src/pixel_target.rs
RUSTDOCFLAGS="-D warnings" cargo test -p device-envoy-core --doc --features host
RUSTDOCFLAGS="-D warnings" cargo doc -p device-envoy-core --no-deps --features host,wasm,doc-images
bash scripts/check-rendered-cyd-links.sh target/doc/device_envoy_core/cyd
just update-docs-core
```

Finally, inspect
`target/doc/device_envoy_core/cyd/display/enum.DrawItem.html` and confirm it
shows exactly one coherent example followed by its rendered bitmap preview.

## Completion criteria

- The `DrawItem` page has one example, not two adjacent examples.
- The example runs as a host doctest and validates its PNG.
- The PNG appears in rustdoc with `doc-images` enabled.
- `PixelTarget` links to this example without copying it.
- No production API or rendering behavior changes.
