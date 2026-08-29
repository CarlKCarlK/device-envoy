# CYD Rustdoc Follow-up Review Spec

<!-- todo0 consider deleting this spec once the accepted documentation changes are implemented and released. -->

This spec collects dispositions from the follow-up CYD rustdoc review. Discuss
each numbered review point before adding or implementing it. Accepted points
record intended documentation changes; implementation remains a separate step.
Rejected point numbers and their reasons belong under **Rejected Changes**.

## Accepted Changes

### 1. Put the drawing-strategy decision table on `CydDisplay`

Add a compact table directly to the portable
[`CydDisplay`](../crates/device-envoy-core/src/cyd.rs) trait documentation so
readers can choose the drawing API and pixel-buffer budget in one place:

| Need | API | Reusable pixel-buffer storage |
| --- | --- | ---: |
| Normal drawing with enough RAM | `full_frame_mut()` | 153,600 bytes |
| Redraw one region | `frame_mut(rectangle)` | 2 × rectangle pixel count bytes |
| Normal drawing with little RAM | `for_each_tile()` | 2 × largest tile pixel count bytes |
| Existing or generated row-major RGB565 pixels | `fill_contiguous()` or `fill_contiguous_full()` | No reusable frame buffer |
| Small immediate `DrawItem` scene | `draw_items()` | No pixel frame buffer |

Explain immediately below the table that the full-screen figure is
`320 × 240 × 2` bytes for the fixed RGB565 panel. Also clarify that
`draw_items` needs allocation-free prepared-item capacity even though it does
not need a pixel frame buffer.

Recommend:

> Start with `full_frame_mut()` when a 153,600-byte frame buffer is practical.

Replace the current direction to start with the `frame_mut` example. Keep the
longer module-level drawing-strategy guide, but make its advice consistent with
the table and avoid repeating the complete table there.

### 2. Explain when to use `DrawItem` instead of embedded-graphics primitives

Expand the [`DrawItem`](../crates/device-envoy-core/src/cyd/display/draw_item.rs)
introduction with the API distinction readers need to choose between the two:

- `DrawItem` is a compact, `Copy` data representation for a heterogeneous
  collection of scene items.
- Its floating-point geometry is convenient for calculated or projected
  coordinates.
- The same items can be passed to `CydDisplay::draw_items`, which composites
  and streams the scene without a pixel frame buffer, or rendered directly
  through `DrawItem::draw` when a frame is available.
- For ordinary imperative drawing into a `CydFrame`, embedded-graphics
  primitives are also appropriate, particularly when their integer-coordinate
  geometry and styling API already fit the scene.

Explain that `DrawItem::draw` uses embedded-graphics internally for strokes and
circles as an implementation detail; `DrawItem` does not compete with or
replace the broader embedded-graphics API. Do not imply that projection is
required to use `DrawItem`. Describe the variants primarily as a filled ellipse
and filled circle. Mention a projected disk or sphere only secondarily if that
application context still adds useful information.

### 3. Make the `draw_items` capacity explicit and user-facing

Rename the public const generic on `CydDisplay::draw_items` from
`PIXEL_SOURCE_COUNT` to `DRAW_ITEM_CAPACITY`. The current name exposes an
internal rendering term and can be mistaken for a pixel count. This follow-up
decision supersedes point #6 in
[`CYD_RUSTDOC_REVIEW_SPEC.md`](CYD_RUSTDOC_REVIEW_SPEC.md), which deferred the
rename during the earlier documentation-only review.

State the sizing rule before readers first encounter a `draw_items` call: each
nondegenerate `DrawItem` consumes at most one prepared-item slot, so setting
`DRAW_ITEM_CAPACITY` to the number of supplied items is always safe. Add an
inline comment to the immediate-operations example:

```rust,no_run
// One DrawItem, so reserve one prepared-item slot.
display.draw_items::<1>(rectangle, Rgb565::BLACK, [
    DrawItem::Circle {
        center: (1.0, 1.0),
        pixel_radius: 1.0,
        color: Rgb888::WHITE,
    },
])?;
```

Give `draw_items` its own focused example if the combined immediate-operations
example still leaves the capacity rule or the no-frame-buffer drawing path
unclear after that edit.

Do not attempt to infer the capacity from the input in this change. The method
accepts arbitrary iterators, including the existing `heapless::Vec` use, rather
than only arrays whose lengths are part of their types. Inference would require
narrowing the API or adding another abstraction or redundant API path.

### 4. Connect drawing strategy to construction on hardware module pages

Bridge the gap between **Choose a drawing strategy** and **Choose a
constructor** on both the ESP and RP CYD module pages. Explain that the drawing
strategy determines the pixel-buffer capacity selected through `new_static`:

> Your drawing strategy determines the pixel-buffer capacity selected through
> `new_static`: use `SCREEN_PIXELS` for full-screen frames, the largest region's
> pixel count for regional frames, `TileGrid::max_tile_pixel_count()` for tiled
> drawing, or `0` for immediate operations and contiguous streaming.

Use the appropriate platform types, methods, and `SCREEN_PIXELS` constants as
links on each page. Say "selected through `new_static`" rather than "passed to
`new_static`": the const generic selects the static storage capacity, and that
storage is subsequently passed to `new`.

Keep this as an early architectural bridge. Do not duplicate the complete
sizing tutorial here; retain the detailed choices and oversize-frame panic
behavior on each platform type's locally complete `new_static` documentation.

### 5. Show the portable CYD abstraction at a glance

Add a compact labeled diagram near the top of the portable `cyd` module page:

```text
CydEsp / CydRp / CydWasm / CydMemory
                  │ implement
                  ▼
                 Cyd
          ┌───────┴───────┐
    parts().0         parts().1
    CydDisplay        CydTouch
         │                 │
 frame_mut()          try_read()
         ▼                 ▼
 CydFrame             TouchEvent
 borrowed frame       calibrated + oriented
```

Accompany the diagram with a brief explanation that `Cyd::parts` borrows the
display and touch components together, `CydDisplay::frame_mut` returns a
temporary borrowed frame for a display region, and `CydTouch::try_read` returns
already calibrated and oriented events. A full-screen frame is the special case
where the borrowed region is the complete display.

Use labeled operations rather than an unlabeled ownership tree: the traits and
returned event are related through borrowing and method results, not necessarily
stored as nested child objects. Keep the diagram on the portable module page;
do not duplicate it across every implementation page.

### 6. Surface the shared touch and drawing orientation guarantee

Place a short, prominent callout near the architecture diagram on the portable
`cyd` module page:

> **Touch-event coordinates and drawing coordinates use the same logical
> orientation. Do not rotate touch points again.**

Use "touch-event coordinates" rather than the less specific "touch
coordinates" so readers do not confuse the portable, calibrated `TouchEvent`
API with internal raw touch-controller samples. Keep the detailed calibration,
orientation, return-value, and bounds explanation on `CydTouch::try_read`; this
early statement advertises the portable API guarantee rather than replacing
those details.

### 7. Define the global frame coordinate and clipping model once

Add a canonical **Coordinates and clipping** explanation to the portable
`CydFrame` documentation:

> **Frames do not introduce a local coordinate system.** All drawing uses
> logical screen coordinates. A frame covering `x = 100..200` and
> `y = 50..100` accepts coordinates in that range and clips drawing outside it.

Explain there that a full-screen frame follows the same rule and merely has its
top-left at `(0, 0)`. Tiled drawing also uses the same model: the application
redraws the same screen-coordinate scene for every tile, while each temporary
frame clips it to that tile.

Keep this as the single complete definition. Add brief links or signposts from
`CydDisplay::frame_mut` and `CydDisplay::for_each_tile` rather than repeating
the full explanation at each method.

### 8. Make the fixed-image pipeline explicit

Add a miniature image pipeline to the `cyd::display` module documentation:

```text
TGA file
   │ tga!()
   ▼
Image888Fixed
   ├── .to_565() ──────────► Image565Fixed
   │                              ├── .view() / .view_rect() ─► Image565View
   │                              └── .at(...).draw[_masked](...)
   └── .to_mask_magenta() ─► MaskFixed ────────────────┘
```

Explain the roles immediately below it:

- `Image888Fixed` owns fixed-size RGB888 source pixels, normally produced at
  compile time by `tga!`.
- `Image565Fixed` owns display-ready RGB565 pixels.
- `Image565View` is a zero-copy borrow of the complete image or a crop, useful
  for contiguous streaming and `DrawItem::Bitmap`.
- `MaskFixed` stores one-bit visibility and is used alongside a matching
  `Image565Fixed` for color-key transparency.
- `MaskedDrawable` is the trait that provides `draw_masked`, not another image
  storage stage.

Show `Image565Fixed` and `MaskFixed` as sibling conversions from the same
`Image888Fixed`, then visually join them at masked drawing. Do not imply that
the mask replaces the RGB565 image.

## Rejected Changes

None.
