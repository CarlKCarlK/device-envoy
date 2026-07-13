<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# DNS Tester Immediate-Mode Layout

## Scope

Refactor the DNS Tester UI into a small, hand-written, typed immediate-mode
layout system modeled on the Linkage Blaze Armatron example. This manual layout
system is the complete deliverable described by this spec.

The implementation should be suitable for a Medium article demonstrating good
embedded Rust patterns:

- the shared game loop owns mutable application state;
- layout is immutable compile-time data;
- rendering mechanics are isolated from application behavior;
- touch hit regions live beside the visual layout they control;
- landscape and portrait are explicit independent layouts;
- target code remains `no_std`, allocation-free, and free of `unsafe`;
- small partial frames provide a deliberate bounded-memory alternative to a
  full-screen framebuffer.

The existing SVG metadata is a design reference for writing and reviewing the
manual Rust constants. Automatically generating Rust from SVG is not required.
It is discussed only as a possible follow-up.

## Motivation

The current rendering code repeats both layout values and drawing mechanics for
every field:

```rust
let rectangle = Rectangle::new(Point::new(22, 76), Size::new(150, 20));
let mut frame = display.frame_mut(rectangle);
DrawItem::Bitmap {
    view: bitmap,
    top_left: Point::new(-rectangle.top_left.x, -rectangle.top_left.y),
}
.draw(&mut frame);
Text::with_text_style(
    target,
    Point::zero(),
    MonoTextStyle::new(&FONT_10X20, Rgb565::from(VALUE_TEXT)),
    TextStyleBuilder::new()
        .alignment(Alignment::Left)
        .baseline(Baseline::Top)
        .build(),
)
.draw(&mut frame)
.unwrap_infallible();
frame.flush().await.map_err(UiError::Display)?;
drop(frame);
```

This mixes four concerns:

- the current application value, such as `target`;
- its layout rectangle and alignment;
- its font and color;
- the mechanics of restoring the background, drawing, flushing, and ending the
  frame borrow.

Touch rectangles are separately embedded in `control_at`, so changing the SVG
design requires finding and editing several unrelated blocks of Rust.

The SVG, `dynamic_layout.md`, and Rust have also drifted. For example, the SVG
metadata describes the latency field as 120x24 while the current Rust frame is
120x29. This spec requires resolving such differences deliberately when the
manual layouts are introduced.

## Linkage Blaze model

The Armatron example separates application state, layout specifications, and
immediate-mode mechanics:

```text
armatron/main.rs
  -> owns game-loop order and changing application values
  -> invokes immediate-mode widgets

armatron/controls.rs
  -> owns immutable widget/layout specifications

examples/ui.rs
  -> owns reusable drawing and touch mechanics
```

DNS Tester should follow the same conceptual model without copying widgets it
does not need. Its button artwork is baked into the bitmap, so it needs dynamic
text slots and tap regions rather than drawn sliders and buttons.

## Target runtime shape

The game loop should select one complete screen description:

```rust
struct Screen {
    bitmap: Image565View,
    layout: Layout,
}

const LANDSCAPE_SCREEN: Screen = Screen::new(LANDSCAPE_BITMAP, LANDSCAPE_LAYOUT);
const PORTRAIT_SCREEN: Screen = Screen::new(PORTRAIT_BITMAP, PORTRAIT_LAYOUT);
```

Orientation selection must choose the bitmap and layout together:

```rust
let screen = match orientation {
    Orientation::Landscape | Orientation::LandscapeInverted => LANDSCAPE_SCREEN,
    Orientation::Portrait | Orientation::PortraitInverted => PORTRAIT_SCREEN,
};
```

This prevents a landscape bitmap from being paired with portrait text or touch
geometry. Inverted orientations share their landscape or portrait layout
because the Cyd abstraction already presents oriented screen coordinates.

The visible game loop should read primarily as application behavior:

```rust
ui.text(screen.layout.target, target).await?;
ui.text(screen.layout.latency, latency.as_str()).await?;
ui.text(screen.layout.queries, query_text.as_str()).await?;
ui.text(screen.layout.successes, success_text.as_str()).await?;
ui.text(screen.layout.failures, failure_text.as_str()).await?;
```

Landscape may additionally draw its status slot:

```rust
if let Some(status) = screen.layout.status {
    ui.status(status, status_text, failures > 0).await?;
}
```

Exact names may change during implementation, but the separation between
application values, immutable layout, and UI mechanics must remain clear.

## Typed layout data

Use the smallest plain-data types that describe the existing UI. A likely
shape is:

```rust
#[derive(Clone, Copy)]
struct TextSlot {
    rectangle: Rectangle,
    font: Font,
    alignment: Alignment,
    color: Rgb888,
}

#[derive(Clone, Copy)]
struct StatusSlot {
    text: TextSlot,
    success_color: Rgb888,
    failure_color: Rgb888,
}

#[derive(Clone, Copy)]
struct TapRegion {
    rectangle: Rectangle,
    control: Control,
}

#[derive(Clone, Copy)]
struct Layout {
    target: TextSlot,
    latency: TextSlot,
    status: Option<StatusSlot>,
    queries: TextSlot,
    successes: TextSlot,
    failures: TextSlot,
    taps: [TapRegion; 3],
}
```

These are design sketches, not mandatory names. During implementation, remove
fields or types that do not make the call sites clearer.

### Font

Use a small closed enum for fonts rather than storing string names:

```rust
#[derive(Clone, Copy)]
enum Font {
    Body,
    Latency,
}
```

`Font` resolves to the corresponding embedded-graphics `MonoFont`. This keeps
layout constants concise and prevents arbitrary font configuration from
leaking into the game loop.

### Control

`Control` describes semantic regions such as calibration, Wi-Fi, and rotation.
It must not embed platform operations such as rebooting or writing flash. The
game loop translates a selected control into its application `Action`.

### Construction

Use direct `const fn` constructors and plain values. Do not use builders,
runtime parsing, heap collections, trait objects, or dynamic dispatch.

## Manual landscape and portrait layouts

Define the layouts explicitly in Rust using values reviewed against the SVGs:

```rust
const LANDSCAPE_LAYOUT: Layout = Layout::new(
    TextSlot::new(rectangle(22, 76, 150, 20), Font::Body, Alignment::Left, VALUE_TEXT),
    // Remaining slots...
);

const PORTRAIT_LAYOUT: Layout = Layout::new(
    TextSlot::new(rectangle(22, 68, 190, 20), Font::Body, Alignment::Left, VALUE_TEXT),
    // Remaining slots...
);
```

The final declarations should be formatted for readability rather than forced
onto one line. Numeric layout values must live in these layout constants, not
in the game loop or rendering implementation.

Landscape and portrait are independent designs. Do not derive one by rotating
or scaling the other. They differ in field presence, alignment, spacing, and
touch geometry.

Both should initially use the same `Layout` type. Landscape-only status can be
represented with `Option<StatusSlot>`. If this makes the runtime harder to
read, separate `LandscapeLayout` and `PortraitLayout` are acceptable, but do
not introduce them preemptively.

## Immediate-mode UI boundary

Introduce a small DNS-specific immediate-mode UI type or an equally clear
focused helper:

```rust
struct Ui<'a, D> {
    display: &'a mut D,
    bitmap: Image565View,
}
```

Its text operation should:

1. create a frame for the slot rectangle;
2. restore that rectangle from the static bitmap;
3. resolve the typed font;
4. draw aligned text using the slot color;
5. flush the rectangle;
6. end the frame borrow before the next operation.

The status operation may reuse the same text primitive while selecting its
success or failure color.

The UI type must not own counters, orientation, target, status, DNS policy,
touch policy, or exit behavior. Like the LB immediate-mode UI, it owns drawing
mechanics and only the ephemeral context required to perform those mechanics.

The UI error should continue to use the LB `derive_more::From` pattern.
Generic display errors remain explicit where coherence prevents a clean
blanket conversion.

## Partial-frame presentation

DNS Tester intentionally differs from Armatron's full-frame rendering.
Armatron owns a full 320x240 RGB565 frame and flushes once per loop. That frame
requires about 150 KiB, which is undesirable beside an embedded Wi-Fi stack.

DNS Tester should continue to use one small frame per dynamic field:

- landscape currently performs six partial flushes per redraw;
- portrait currently performs five partial flushes per redraw;
- each field restores its rectangle from the static bitmap before drawing;
- fields do not overlap;
- independently timed field updates and temporary visual tearing are acceptable
  for this application.

This tradeoff should be explicit in the implementation and article: more
display transactions buy bounded memory usage. An immediate-mode abstraction
does not require a full-screen framebuffer or one flush per loop.

Batching adjacent fields is outside this spec. Do not add batching machinery
without measurements showing that it is needed.

## Touch handling

Tap regions belong to `Layout` beside the text slots. The button artwork is
already in the bitmap, so these regions perform hit-testing without redrawing
buttons.

Input handling should be equivalent to:

```rust
let control = screen
    .layout
    .taps
    .iter()
    .find(|tap_region| tap_region.rectangle.contains(point))
    .map(|tap_region| tap_region.control);
```

The three regions must not overlap. Because they are disjoint, array order must
not affect behavior. Add a test that proves the hand-written layout satisfies
this invariant.

An ordinary touch outside all control regions continues to start a DNS lookup.
Touch calibration and orientation mapping remain upstream of layout
hit-testing.

## Relationship to the SVG designs

For this implementation, the SVGs are design documents and bitmap inputs, not
code generators. Their existing comments and `data-*` attributes should be
used to review the hand-written constants.

When introducing the layouts:

- compare every text slot and tap region against the corresponding SVG;
- resolve the 24-versus-29 latency-height difference deliberately;
- update inaccurate SVG comments or metadata when the Rust behavior is the
  intended design;
- update Rust when the SVG is the intended design;
- keep comments descriptive rather than treating prose as a parser contract.

`dynamic_layout.md` must be updated to match the chosen values or reduced to
narrative workflow documentation. It must not knowingly disagree with the
implementation at the end of this work.

Manual synchronization is accepted by this spec. Build-time enforcement is a
possible future improvement, not an acceptance requirement.

## Item organization

Keep the primary DNS Tester entry points near the top of `dns_tester.rs`.
Follow them with error and exit types, layout/UI types, constants, and helper
functions according to repository conventions.

If the layout and immediate-mode implementation make `dns_tester.rs` difficult
to read, use the repository module pattern:

```text
src/dns_tester.rs
src/dns_tester/layout.rs
src/dns_tester/ui.rs
```

Do not create `mod.rs`. Prefer keeping a small implementation in one file over
creating submodules that contain only a few declarations.

## Implementation phases

### Phase 1: Introduce typed manual layouts

Add the smallest useful `Screen`, `Layout`, text-slot, status-slot, font, and
tap-region types. Define landscape and portrait layouts manually from the
current design.

Bundle each layout with its matching bitmap. Add const or host tests for screen
bounds and non-overlapping tap regions.

### Phase 2: Introduce immediate-mode rendering

Move bitmap restoration, aligned text drawing, frame flushing, and borrow
termination into the DNS-specific UI boundary.

Replace the devolved rendering blocks with concise calls using the selected
screen's layout. Preserve per-field flushes.

### Phase 3: Use layout tap regions

Delete the independent coordinate table in `control_at`. Resolve taps through
the selected layout and preserve existing CAL, WI-FI, ROTATE, ordinary tap,
and BOOT behavior.

### Phase 4: Validate behavior and clean up

Resolve metadata drift, update CydMemory goldens if the intended design
changes, remove obsolete constants/helpers, and review the resulting game loop
as teaching material.

The complete implementation stops after this phase.

## Testing

Add tests for:

- every text rectangle being contained within its screen;
- every tap rectangle being contained within its screen;
- non-overlapping tap regions;
- exactly one calibration, Wi-Fi, and rotation control in each layout;
- landscape and landscape-inverted selecting the landscape screen;
- portrait and portrait-inverted selecting the portrait screen;
- successful and failed status colors;
- all three controls in both layout families;
- an ordinary non-control tap starting a DNS lookup;
- boundary points following embedded-graphics rectangle containment semantics;
- shorter replacement text restoring its complete bitmap rectangle first;
- existing landscape and portrait CydMemory golden images.

Tests should execute the same shared runtime and immediate-mode code used by
ESP, RP, and WASM.

## Acceptance criteria

- The game loop contains no literal dynamic-field or touch rectangles.
- `LANDSCAPE_SCREEN` and `PORTRAIT_SCREEN` each bundle a matching bitmap and
  hand-written typed layout.
- Landscape and portrait remain explicit independent layouts.
- Dynamic text is rendered through a small DNS-specific immediate-mode API.
- Touch hit-testing uses regions from the selected layout.
- The UI layer owns mechanics but no DNS application state or policy.
- Per-field partial flushing and bounded memory behavior are preserved.
- Target runtime code remains `no_std`, allocation-free, and free of `unsafe`.
- ESP, RP, WASM, and CydMemory continue to call the same shared runtime.
- CydMemory goldens and touch tests cover both layout families.
- SVG metadata, `dynamic_layout.md`, and the chosen Rust values do not knowingly
  disagree when implementation is complete.
- The visible runtime is concise enough to explain as an example of immutable
  typed layout, immediate-mode rendering, shared platform-neutral application
  logic, and bounded-memory embedded design.

## Possible follow-up: SVG-generated layouts

This section is intentionally non-binding. It is not part of implementation or
acceptance for this spec.

If manual synchronization becomes burdensome, `build.rs` could parse stable
SVG `data-*` attributes and emit constants with exactly the same `Layout` and
`Screen` types established here:

```text
dns_landscape.svg -> dns_landscape.tga + LANDSCAPE_LAYOUT
dns_portrait.svg  -> dns_portrait.tga  + PORTRAIT_LAYOUT
```

The runtime API and game loop should not change. Generation would replace only
the source of the constants.

A future generation spec should separately define:

- the SVG metadata schema;
- typed build-time parsing;
- validation and diagnostics;
- deterministic generated Rust;
- ownership of duplicated SVG guides and `dynamic_layout.md`;
- tests for malformed and incomplete metadata.

Do not implement this follow-up merely because the manual types make it
possible. Revisit it only after experience changing the hand-written layouts.

## Non-goals

This spec does not require:

- generating Rust from SVG;
- parsing SVG comments or metadata at build time;
- a general-purpose GUI framework in `device-envoy-core`;
- runtime SVG or XML parsing;
- heap allocation on embedded targets;
- automatic rotation or scaling between landscape and portrait;
- one full-screen framebuffer;
- one flush per complete game-loop iteration;
- batching partial updates;
- overlapping widget z-order or hit-test arbitration;
- changing DNS, Wi-Fi, calibration, persistence, or reboot policy.
