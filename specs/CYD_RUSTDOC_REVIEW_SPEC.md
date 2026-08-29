# CYD Rustdoc Review Spec

<!-- todo0 consider deleting this spec once the accepted documentation changes are implemented and released. -->

This spec collects dispositions from the CYD rustdoc review. Discuss each
review point before adding or implementing it. An accepted point records the
intended documentation change; implementation remains a separate step.

## Accepted Changes

### 2. Make `CydFrame::Error` platform-neutral

The portable [`CydFrame`](../crates/device-envoy-core/src/cyd/display.rs) trait
currently documents its associated error type as:

> Error returned when flushing this frame to the panel.

That wording incorrectly assumes physical display hardware. `CydFrameMemory`
presents into the in-memory harness, and `CydFrameWasm` presents to an HTML
canvas.

Change the trait documentation to:

> Error returned when presenting the frame.

`CydFrame::Error` is used only as the error returned by `CydFrame::flush`, whose
portable operation is already described as frame presentation. The new wording
therefore covers hardware SPI failures, in-memory frame-budget failures, and
infallible browser presentation.

Retain the `CydFrameWasm` implementation override:

> WASM frame presentation is infallible.

It communicates a useful platform guarantee even after the inherited trait
wording becomes accurate.

### 4. Keep one complete `CydDisplay::frame_mut` example

Replace the two current code blocks with one generic example that creates a
frame over a non-zero-origin rectangle and chains `fill`, `write_text`, and
`flush`. The visible example must introduce `display` rather than relying on
hidden host-only setup.

Keep the visual preview. Update its golden-image test to render the same blue
100×40 region containing `CYD`, then regenerate the expected PNG so the example
and preview agree.

### 5. Make the `fill_contiguous_full` example orientation-independent

Derive the iterator's row and column ranges from `display.screen_size()` rather
than hard-coding a 320×240 landscape layout. Scale the generated RGB565 color
components by those dimensions so every component remains in range in both
orientations.

Update the corresponding golden-image helper to accept the screen size and use
the same calculation, then regenerate its preview PNG.

### 6. Use ordinary 2D terminology for `draw_items`

Describe the method as drawing `items` immediately inside `bounds`; do not call
the items projected.

Explain that `PIXEL_SOURCE_COUNT` is the allocation-free capacity for prepared
draw items, that each nondegenerate item consumes at most one slot, and that an
item can consume a slot even when it lies outside `bounds`. State that using the
total supplied item count is always safe, and add a `# Panics` section for
capacity exhaustion.

Do not rename the const parameter as part of this documentation review. A
public API rename requires a separate decision.

### 7. Remove callback and lifetime jargon from tiled drawing docs

Describe `CydDisplay::for_each_tile` in terms of its `draw` parameter and
observable ordering:

- `draw` receives one logical-display-coordinate frame for each tile.
- Each frame clips drawing to its tile.
- The frame is flushed after `draw` returns and before the next tile is
  processed.
- Only one tile is buffered at a time.

Remove the public explanation of lending iterator lifetimes. Replace
`callback-tiling` in the `cyd::display` module description with ordinary tiled
drawing terminology.

### 8. Prune the `TileGrid` example and accessor links

Keep the visual tiled drawing and
`assert_eq!(GRID.max_tile_pixel_count(), 80 * 80)`. Remove `FRAME_PIXELS` and the
assertions for `rectangle`, `columns`, `rows`, `tile_width`, and `tile_height`.

Remove the repetitive example links from those five self-explanatory accessors.
Retain links from `TileGrid::new` and `max_tile_pixel_count`, because the final
example still demonstrates construction, tiled drawing, and buffer sizing.

Audit every `TileGrid` method link after editing: a method must not refer to the
example unless the final example actually demonstrates that method.

### 9. Remove repeated links from `TouchEvent` variants

Keep the calibrated, oriented logical-coordinate rule and the focused
`CydTouch::try_read` link at the `TouchEvent` type level. Reduce the variant
documentation to:

- `Down`: The touch contact began at `point`.
- `Move`: The active contact moved to `point`.
- `Up`: The touch contact ended.

Document each public `point` field simply as the contact position. Do not repeat
the example link or coordinate rule on the variants and fields.

### 10. Prune the `CydTouch` introduction and error docs

Reduce the trait introduction to:

> A CYD touch source that returns calibrated, oriented touch events in logical
> display coordinates.

Document the associated error as:

> Error returned when reading touch input.

Keep the complete `try_read` return semantics, application-example link, and
warning against applying orientation twice. Retain the hidden `handle_point`
doctest scaffolding; it keeps the visible example focused while ensuring the
example compiles.

### 11b. Focus the secondary `CydMemory` docs and audit links

Keep the comprehensive main `CydMemory` host example. Its combined touch,
button, drawing, flush, state-inspection, and golden-image workflow demonstrates
how the harness components share backing state and frame progression.

Apply only the following focused cleanup:

- Remove or retarget links from `owned_parts`, `last_flush_rectangle`, and
  `pixel` so none claims to be demonstrated by an example that does not use it.
- Focus `new_with_orientation` on orientation, framebuffer rotation, and pixel
  inspection.
- Use an inverted orientation when demonstrating `rotate_framebuffer_180`.
- Move frame-budget exhaustion into a focused `set_frame_budget` example.

### 12. Prune repetitive `CydMemory` example links

Remove the repeated generic “See the example on `CydMemory`” sentences. Keep a
link only when the linked example visibly uses the documented item and adds
information beyond the method's own description.

Keep or rewrite links contextually for:

- `new`, which leads to the canonical construction and host-test workflow;
- `button_memory`, which demonstrates shared frame-clock behavior;
- `push_touch_event`, which demonstrates injection followed by portable touch
  reading;
- `set_pressed_for_frame`, which demonstrates scheduled state changing after a
  flush;
- `rotate_framebuffer_180`, which leads to the focused inverted-orientation
  example; and
- `assert_framebuffer_matches_expected_png`, which leads to the complete
  golden-image workflow.

Remove generic example links from `display`, `owned_parts`, `flush_count`,
`last_flush_rectangle`, and `pixel`. Give `set_frame_budget` its own focused
example under #11b rather than linking it to `new_with_orientation`.

Where a link remains, explain what the linked example demonstrates instead of
using a generic cross-reference sentence.

### 13b. Keep each `new_static` page locally complete

Do not establish a single static-buffer sizing authority for an entire platform
family. Readers should not have to move between the two-SPI and one-SPI type
pages to understand construction.

Give each distinct public `new_static` page a concise, self-contained
explanation of the available `PIXEL_COUNT` choices:

- `0` for immediate operations and contiguous streaming without buffered
  frames;
- a regional buffer sized for the largest requested frame;
- a tiled buffer sized with `TileGrid::max_tile_pixel_count`; and
- a full-screen buffer sized with the type's `SCREEN_PIXELS` constant.

This applies independently to `CydEsp::new_static`,
`CydEspOneSpi::new_static`, `CydRp::new_static`, and
`CydRpOneSpi::new_static`. Each constructor must also state that requesting a
frame or tile larger than its buffer panics.

Avoid repeating those sizing choices elsewhere on the same device type page.
The corresponding `new` method should briefly explain that its static argument
controls RAM use and buffered-region capacity, then link to that type's own
`new_static` method.

Simplify the separate static-storage type pages to describe the storage they
contain, identify `PIXEL_COUNT` as an RGB565 pixel count rather than a byte
count, and retain a focused declaration example. They may link to the relevant
`new_static` method or methods, but should not repeat the complete sizing and
tiling tutorial.

In particular, retain the valuable `CydRpOneSpiStatic` explanation that the
storage also contains the shared-bus mutex and that its SPI type parameter `T`
must match the selected Embassy SPI peripheral. Remove the duplicated
pixel-budget tutorial from that type page.

Reverse any current cross-reference claiming that a static-storage type is the
authoritative source for complete sizing rules. The locally complete
`new_static` method is the reader-facing sizing authority for its device type.

### 14. Remove constructor-example test assertions

Remove the final
`assert_eq!(cyd.orientation(), Orientation::Landscape)` assertion from the
`CydRp::new` and `CydRpOneSpi::new` examples. Remove the equivalent hidden
assertions from the `CydEsp::new` and `CydEspOneSpi::new` doctest scaffolding
for consistency.

The examples already pass `Orientation::Landscape` visibly to the constructor,
and no documentation link relies on these examples to demonstrate the
`orientation` getter. End each example after successful construction and
`Ok(())`. Retain the grouped argument comments that make the long constructor
calls readable.

### 15. Make the ESP and RP frame pages intentionally sparse

Replace the type-level `CydFrameEsp` and `CydFrameRp` tutorials with concise
descriptions identifying each type as its platform's implementation of
`CydFrame`. Explain that frames are returned by `CydDisplay::frame_mut`, and
direct readers to the portable `CydFrame` documentation for normal drawing.

Remove the concrete-type examples, the “This page contains the ... example”
wording, and method links that point back to those removed examples. Retarget
`fill` and `write_text` to the portable documentation where a link adds useful
information. Give `width`, `height`, and `raw_pixels_mut` direct,
self-contained descriptions without manufacturing replacement example links.

Retain genuinely platform-specific behavior on the relevant method. In
particular, continue to explain that the inherent `flush` synchronously writes
the buffered rectangle over SPI, while portable `CydFrame::flush` exposes the
async frame-presentation boundary.

### 16b. Consolidate low-value example links without weakening doctest promises

Perform a global audit of repetitive “See ... example” links, with particular
attention to the WASM browser-shell API. Do not apply a blanket rule that a
type-level link always replaces links on its members: a direct link on a public
method is a useful promise that the linked doctest actually exercises that
method.

For plain data fields and enum variants shown together on the same rustdoc page,
remove identical field- or variant-level links when one type-level statement
can describe the coverage accurately. Make that statement explicit rather than
offering a generic cross-reference. For example:

> The compiled browser-shell example constructs and reads every field.

Apply this consolidation where accurate to `Config`, `PageInfo`,
`Capabilities`, `Command`, and `NoticeSeverity`. Preserve each field's or
variant's direct semantic description.

Keep direct example links on public methods when the linked doctest visibly
calls that method. This includes methods on browser-shell types such as
`Handle`, and constructors such as `Config::new` and `PageInfo::new`. Remove or
retarget a method link only when the example does not demonstrate that method
or the same item already contains a redundant link.

After pruning, verify every type-level coverage statement and every surviving
member link against the compiled doctest. A coverage statement must not claim
that all fields or variants are demonstrated unless the example actually uses
all of them.

## Rejected Changes

- #3
- #11

## Validation

After implementing accepted changes:

1. Run `just docs` from the workspace root.
2. Inspect the rendered `CydFrame`, `CydDisplay`, `TileGrid`, `TouchEvent`,
   `CydTouch`, and `CydMemory` pages, including their visual previews and
   implementation-specific inherited text.
3. Regenerate the agent corpus with `just docs-agent-text`.
4. Confirm the corpus contains no stale `projected draw items`,
   `callback-tiling`, or reader-visible lending-iterator wording.
5. Confirm every surviving example link points to an example that visibly uses
   the documented item.
6. Inspect all four `new_static` pages and their static-storage type pages.
   Confirm each constructor page is locally usable, sizing choices are not
   duplicated elsewhere on the same device type page, and the RP one-SPI page
   retains its shared-bus and SPI-type explanation.
7. Inspect the four ESP/RP `new` examples and confirm none ends with a test-like
   orientation assertion and all retain their grouped argument comments.
8. Inspect `CydFrameEsp` and `CydFrameRp`. Confirm their duplicate concrete
   tutorials and stale method links are gone, while their synchronous SPI
   `flush` distinction remains clear.
9. Audit the WASM browser-shell pages and compiled module example. Confirm
   repeated field and variant links have been consolidated, explicit type-level
   coverage claims are accurate, and every surviving public-method link leads
   to a doctest that visibly calls that method.
