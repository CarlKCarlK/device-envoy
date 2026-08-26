<!-- TODO0 Implement this CYD public-API cleanup before the next release, then consider deleting this spec. -->

# CYD Public API Cleanup

The ESP and RP CYD modules expose several implementation types alongside the
device abstractions that applications use. Reduce that surface before the next
release while keeping the ESP and RP APIs parallel where their hardware allows.

## Keep Public and Documented

- The complete-device types: `CydEsp`/`CydRp` and their one-SPI alternatives.
- The concrete display and calibrated-touch component types.
- The frame and static-storage types returned or required by public APIs.
- The `Cyd`, `CydDisplay`, and `CydTouch` traits.
- `Orientation`, `tiling`, `touch`, `DEFAULT_DISPLAY_SPI_HZ`, and `DEFAULT_FONT`.
- The narrow `cyd::backend::DisplayBackend` seam required by the separate
  ESP/RP platform crates; it is backend-only and is not re-exported by them.
- One module-level `Error` type that preserves useful failure diagnostics.
- Keep display and touch as separate concerns: display owns frames, drawing,
  orientation, and tiling; touch owns input events and calibration. The
  complete-device abstractions coordinate them whether the hardware uses
  separate SPI peripherals or one shared bus.

## Remove From the Public Surface

- Make `PixelBuffer` private; it is storage hidden behind `CydStaticEsp` and
  `CydStaticRp`.
- Delete `RegionBuffer` if a final usage audit confirms that no supported
  application path needs it.
- Remove or privatize `CydFrameEsp::view_mut` and `CydFrameRp::view_mut` if
  `DrawTarget`, `fill`, and `raw_pixels_mut` cover the supported use cases;
  then make `RegionView` private.
- Make `TOUCH_SPI_HZ` private because touch constructors do not expose clock
  selection as policy. It is a fixed implementation setting, not a default;
  do not rename it to `DEFAULT_TOUCH_SPI_HZ`, which would falsely imply that
  applications can select an alternative through the supported API.
- Remove the uncalibrated platform touch types from the application-facing
  surface. Reshape the shared calibration traits and errors as needed rather
  than hiding public types from rustdoc.

## Calibration Boundary

Calibration is constructor behavior, not an application subsystem. The
supported ESP and RP workflow is: construct a complete device, allow the
constructor to load or run calibration, and read screen-space `TouchEvent`s
through `CydTouch::read`. Applications must not need to assemble or drive the
calibration state machine.

- Remove the public `touch::calibration` module. Keep the calibration math,
  persistence format, drawing, validation, and state machine private.
- Reduce `CydTouch` to the calibrated operations applications use. Remove its
  `Uncalibrated` associated type, `calibration_config`, and `decalibrate` unless
  a supported downstream application demonstrates a need for manual
  calibration ownership.
- Add a deliberately small `device_envoy_core::cyd::backend` module for authors
  of platform crates. Rust has no cross-crate `pub(crate)` or friend
  visibility, so ESP and RP require a real public seam to reuse the core
  calibration implementation. This is a backend-author API, not an
  application calibration API.
- Keep only the unavoidable cross-crate items public in `cyd::backend`:
  `TouchUncalibrated`, `RawTouchEvent`, `CalibrationConfig`,
  `ensure_calibration`, and one module-scoped `Error`. Use concise names because
  the `backend` module supplies their context.
- Retain the narrow `backend::DisplayBackend` display-construction seam required
  by the separate ESP and RP crates. It is backend plumbing, not an
  application-facing drawing API, and must not be re-exported by those crates.
- Simplify `ensure_calibration` to return the calibrated touch value directly.
  Remove the public `EnsureCalibrationOutcome`; ESP and RP currently discard
  it. Fold device and flash failure variants directly into `backend::Error`
  rather than exposing a second `ErrorKind` layer.
- Keep `RawPoint`, `CalibrationFlow`, `CalibrationValidation`,
  `CalibrationCorner`, `EnsureCalibrationSettings`, all calibration tuning
  constants, geometry and drawing helpers, solve and validation functions, and
  alternative driver entry points private behind `cyd::backend`.
- Keep the concrete platform uncalibrated touch types private. They implement
  `backend::TouchUncalibrated` only so the complete-device constructors can
  call `backend::ensure_calibration`.
- Do not re-export `cyd::backend` or any of its items from ESP or RP. Their
  public `touch` modules expose only application-facing calibrated touch items,
  including `TouchEvent`.
- Preserve the serialized calibration data and automatic flash-backed behavior
  behind the complete-device constructors. Moving `CalibrationConfig` must not
  silently invalidate already stored calibration without an intentional
  format/version decision.
- Do not use generated copies, duplicated calibration engines, a large exported
  macro, or `#[doc(hidden)]` merely to make the unavoidable backend seam appear
  private. Prefer the small honest backend API over substantial maintenance
  machinery for marginal documentation purity.

The workspace examples and the ESP/RP starter projects currently use only the
complete-device constructors and calibrated touch reads. They do not name the
backend seam or any calibration flow, configuration, validation, geometry,
drawing, settings, outcome, or error items. Re-run that downstream audit during
implementation in case usage changes.

## Drawing API Hierarchy

The retained drawing API must present a clear hierarchy rather than several
peer-level buffering strategies. The public mental model should cover the most
useful application workflows with the least machinery: normal applications
draw frames, memory-constrained applications use a helper that owns tiling, and
advanced applications may stream pixels directly.

1. The normal application workflow is frame-based:
   - `full_frame_mut()` for the whole display
   - `frame_mut(rectangle)` for a region
   - draw into the returned frame and flush it

2. The supported low-memory workflow is tiled drawing. Prefer a helper such as
   `for_each_tile(grid, draw)` that owns tile iteration and flushing:

   ```rust,no_run
   display
       .for_each_tile(grid, |frame| {
           draw_scene(frame);
       })
       .await?;
   ```

   The callback is invoked once per tile and therefore must be synchronous and
   replayable. Document this explicitly. Do not silently replay one-shot
   iterators or state-changing application logic.

3. Contiguous-pixel streaming is an advanced fast/low-memory path and must be
   documented separately from the normal frame workflow rather than presented
   as an equal starting point. Retain both `fill_contiguous(rectangle, pixels)`
   for regional streams and `fill_contiguous_full(pixels)` for whole-screen
   streams. The full-screen convenience earns its place by expressing intent
   directly and avoiding repeated construction of
   `Rectangle::new(Point::zero(), display.screen_size())`; Linkage Blaze's
   full-screen clock splash is the demonstrated application use.

Audit the existing methods accordingly:

- Keep `frame_mut_with_tile_top_left` out of the application-facing story. It
  remains only as the narrow `cyd::backend::DisplayBackend` seam required by
  the separate ESP and RP platform crates.
- Prefer `for_each_tile`; the lending `Tiles` iterator and `CydDisplay::tiles`
  are private implementation details and are not part of the application API.
- Retain `fill_contiguous_full` as the application-facing whole-screen
  streaming convenience. Audit `flush_at` and `draw_items` for redundancy or
  backend-only use, retaining them publicly only when a demonstrated
  application use justifies them.
- Do not add a callback helper while retaining every existing peer-level path
  by default. The objective is a smaller mental model and public surface.
- Do not add automatic `draw_screen` strategy selection until buffer-capacity
  behavior and repeated-drawing semantics are clearly defined and tested.

### Drawing Strategy Examples

Before finalizing the hierarchy, document one canonical, coordinate-sensitive
scene rendered through every retained drawing strategy:

1. one full-screen frame;
2. independently updated regional frames;
3. callback-based tiled rendering; and
4. contiguous row-major pixel streaming.

The documentation must explain why a caller chooses each strategy, what buffer
or replay behavior it requires, which coordinate space its drawing operations
use, and what it gives up compared with the normal frame workflow. Full-screen
and regional frames are one coherent frame family, but the examples must still
show their different whole-scene and independently updated-region use cases.
Tiling must explain that it replays a synchronous scene once per tile.
Streaming must be presented as a distinct advanced raster path rather than as a
general drawing-target replacement.

Use a scene with asymmetric landmarks and content crossing region and tile
boundaries so incorrect origins, clipping, row order, or incomplete coverage
cannot accidentally look correct. Keep focused compiled `rust,no_run` doctests
on the individual APIs, and make them link directly to the applicable section
of the canonical comparison. The examples should use application-facing APIs
only. If they require backend plumbing, manual tile-coordinate translation, or
knowledge of internal buffers, treat that as evidence that the API still needs
design work rather than hiding it as doctest boilerplate.

Back the documentation with host tests that render each strategy into a
separate `CydMemory` device and assert that all four final framebuffers are
identical. Where useful, also assert the full-frame, largest-region, and tile
buffer sizes so the examples demonstrate the memory tradeoff rather than only
producing the same pixels. The four call sites need not look artificially
identical; their genuine tradeoffs should be obvious while their output remains
equivalent.

Linkage Blaze is a required downstream migration target for this phase, not
merely an optional compatibility check. Its shared CYD examples exercise each
important level of the hierarchy: `examples/skeleton_clock.rs` uses explicit
tiling, `examples/clock.rs` uses `fill_contiguous_full` and `draw_items`, and
the ballet and armatron examples use full frames. Its ESP, RP, and WASM example
crates also exercise full-screen and regional-frame construction. Update these
call sites to the final CYD API in the same change, preserving their intended
memory use and rendering behavior. Do not retain Device Envoy compatibility
aliases or redundant drawing paths solely to avoid updating Linkage Blaze.

## Consolidate Errors

Fold the platform-specific display-init, display-flush, and touch-init label
enums into each platform module's `cyd::Error`. The current nested enums add
names without retaining underlying source errors, and the crate-wide error
type wraps them again. Keep enough variant detail to distinguish configuration,
panel initialization, frame flushing, and orientation failures. Retain a
SPI-device-creation failure only where construction is actually fallible; use
`unwrap_infallible()` instead of inventing an impossible public error variant
for infallible GPIO chip-select pins.

Every public error type must be named `Error`; use its module path to
distinguish it from other error types. Do not expose names such as `ErrorKind`,
`CydDisplayEspInitError`, `CydDisplayEspFlushError`,
`CydTouchEspInitError`, `CydDisplayRpInitError`,
`CydDisplayRpFlushError`, or `CydTouchRpInitError`. Remove all six platform
types and their root-level re-exports. Prefer one `cyd::Error` on each
platform. If a truly independent public submodule needs its own error, name it
`Error` so callers refer to it through that module, for example
`display::Error`, rather than encoding the module and operation into the type
name. Use variants on `Error` to distinguish failure categories and store the
original source errors; do not preserve the current label-only unit enums as
nested payloads.

The calibration `ErrorKind` is not useful publicly. Put device and flash
variants directly on `backend::Error`; the ESP/RP constructors then translate
that error into the platform's `cyd::Error` while preserving the underlying
diagnostic.

## Documentation Rules

- Audit the rendered public items recursively on every ESP, RP, and core CYD
  page and subpage. Each item must have a demonstrated application use or be
  part of the deliberately minimal platform-backend seam; accidental
  cross-crate convenience alone is not sufficient justification.
- Do not use `#[doc(hidden)]` to conceal ordinary implementation details; make
  them private or redesign the public signature that leaks them.
- The only hidden-public exception is a helper that downstream expansion of a
  public macro must name. Prefix such a helper with `__` and document beside
  it why Rust visibility requires it to remain public. No current calibration
  item qualifies for this exception.
- Write re-exported-item documentation from the application's view. Link text
  must use the public item name, never an implementation-relative path such as
  `super::CydDisplayEsp::new`. In particular, document `DEFAULT_FONT` as the
  default font accepted by `CydDisplayEsp::new`/`CydDisplayRp::new`, with those
  public method names as the link text. Audit both platforms together.
- Keep `CydTouch` public and visible because applications import it to call
  calibrated touch operations such as `read`.
- Keep the ESP and RP module introductions fully linked: `CydTouch` and the
  referenced `device_envoy_core::cyd` module must render as links rather than
  bracketed code. Audit both platform pages whenever either introduction or
  the core trait exports change.
- Keep core, ESP, and RP release versions and dependency requirements aligned
  so docs.rs cross-crate links resolve to the release that provides the
  documented CYD traits. Publish core before the platform crates.
- Ensure the core CYD module page lists and links every application-facing CYD
  trait re-exported by the platform modules, including `CydTouch`.
- Do not link public documentation to a `#[doc(hidden)]` item. In particular,
  the ESP, RP, and core touch-module introductions must not render
  the former `CydTouchUncalibrated` as bracketed code. Application-facing docs
  must link only to the calibrated API; backend docs link to the renamed
  `backend::TouchUncalibrated` where required.
- Document `cyd::backend` tersely and honestly: applications should use `Cyd`
  and `CydTouch`; the module is public because ESP and RP are separate crates
  that reuse the core implementation. Keep one compiled backend example and
  make each backend item link directly to it rather than repeating extensive
  calibration documentation.
- Keep one canonical compiled `rust,no_run` tiling example on
  `CydDisplay::for_each_tile`. Make the `tiling` module, `TileGrid`, and the
  public rectangle-size helpers either contain a focused compiled example or
  link directly to that canonical example. The internal `Tiles` iterator is
  not part of the retained public checklist.
- Apply the same example-or-direct-link rule to the major public items in
  `touch`; embedded examples may be `rust,no_run`, but they must compile.
- Keep module introductions and public surfaces parallel between ESP and RP.
- Use the multiplication sign in prose dimensions, for example `320×240`.

## Doctest Audit

Audit pages after the visibility cleanup, rather than spending documentation
work on items that this spec removes. Every retained public module, type,
trait, function, constant, and public method must contain a compilable
`rust,no_run` example or link directly to a compilable `rust,no_run` example
that exercises or explicitly mentions that item. A link to a type or module
page is not sufficient merely because some unrelated example appears elsewhere
on that page.

The current `Cyd` example does not satisfy this rule on published embedded
docs: it is added only by `#[cfg_attr(feature = "host", doc = ...)]`, while
docs.rs builds the ESP/RP-facing core documentation without `host`. The same
problem affects the `CydDisplay::screen_size` and `CydDisplay::frame_mut`
examples. Add unconditional, platform-neutral canonical examples for the
public traits. The backend-only frame-construction seam is not an
application-facing `CydDisplay` item. Host-only framebuffer examples and
preview images may remain as supplemental documentation, but public
documentation must not depend on them.

Use this retained-surface checklist:

- `cyd`: link to the canonical `Cyd` example and the platform constructor
  examples.
- `Cyd`: add one unconditional generic device-loop example. Make `parts`,
  `display`, and `orientation` link directly to it.
- `CydTouch` and `TouchEvent`: add or link directly to the canonical calibrated
  touch-read example. They must not lead application readers into `backend`.
- `backend`, `TouchUncalibrated`, `RawTouchEvent`, `CalibrationConfig`,
  `ensure_calibration`, and `backend::Error`: share one concise compiled
  platform-implementation example, with every item linking directly to it.
  Private calibration machinery needs no doctests.
- `CydDisplay`: keep focused canonical examples for the getter family, frame
  creation and flushing, contiguous drawing, immediate fill/draw operations,
  and tiling. Every retained method must link to the specific example that
  covers it; generic links such as "the trait documentation" do not suffice.
- `display` and `CydFrame`: link to canonical examples that actually exercise
  their primary types and every retained method. `RectanglePixels` was removed
  during the drawing cleanup and is not a retained public item.
- `DrawItem`/`Image565View`, `Orientation`, and the retained compile-time TGA
  image/mask types, functions, and macro: provide one focused canonical example
  per coherent family, with every family member and method linked directly to
  it.
- `tiling`, `TileGrid`, `rectangle_pixel_count`, and
  `max_rectangle_pixel_count`: use `CydDisplay::for_each_tile` as the
  canonical draw loop and add a focused grid/buffer-sizing example where
  needed. The internal `Tiles` iterator and its `next` method need no public
  documentation audit.
- On ESP, cover `CydEsp`, `CydEspOneSpi`, `CydDisplayEsp`, `CydStaticEsp`,
  `CydFrameEsp`, `CydTouchEsp`, the retained `cyd::Error`,
  `DEFAULT_DISPLAY_SPI_HZ`, and `DEFAULT_FONT`. Constructor and static-storage
  examples may be canonical for the related type, constants, and returned
  components when each page links to them explicitly.
- Apply the identical audit to RP: `CydRp`, `CydRpOneSpi`,
  `CydRpOneSpiStatic`, `CydDisplayRp`, `CydStaticRp`, `CydFrameRp`,
  `CydTouchRp`, the retained `cyd::Error`, `DEFAULT_DISPLAY_SPI_HZ`, and
  `DEFAULT_FONT`.
- Audit `SCREEN_PIXELS` and every retained associated constant by linking them
  to the static-storage example that uses them.

The published pages previously failed this audit broadly. Treat the checklist
as the starting audit, then re-inventory the rendered pages after visibility
changes so newly retained items cannot escape it.

## Implementation Order

Implement this cleanup in the following phases. Complete and review each phase
before starting the next so documentation work targets only the final public
surface.

### 1. Platform Surface and Errors

- Apply the non-calibration visibility cleanup to ESP and RP: buffers, frame
  escape hatches, touch clock policy, and related re-exports.
- Consolidate the prefixed ESP/RP error types into each platform's
  module-scoped `cyd::Error`, preserving useful source diagnostics.
- Keep the ESP and RP public surfaces parallel and run focused platform checks.

### 2. Calibration Boundary

- Implement the narrow `device_envoy_core::cyd::backend` boundary specified
  above and simplify the application-facing touch traits.
- Move only the five required backend items into that module; privatize the
  remaining raw-point, flow, validation, geometry, drawing, settings, outcome,
  and driver machinery.
- Stop re-exporting backend support from ESP and RP. Keep their concrete
  uncalibrated types private and their application-facing touch modules clean.
- Preserve automatic interactive calibration, flash persistence, and stored
  calibration compatibility unless an intentional migration is documented.
- Run focused core, ESP, and RP checks and review this architectural phase
  before proceeding.

### 3. Documentation and Doctests

- Apply the documentation rules and retained-surface doctest checklist only
  after phases 1 and 2 establish the final API.
- Build and inspect core, ESP, and RP rustdoc using the publication feature and
  target combinations.
- Fix broken, relative, hidden-item, and cross-crate links while keeping the
  platform documentation parallel.

### 4. Drawing API Hierarchy

- Audit downstream and example use of the backend-only
  `frame_mut_with_tile_top_left`, internal `Tiles`, `for_each_tile`,
  `fill_contiguous_full`, `flush_at`, and `draw_items`. Include the sibling
  Linkage Blaze workspace and classify its shared and platform-specific CYD
  examples.
- Restore and retain `fill_contiguous_full(pixels)` as the concise full-screen
  streaming operation, and keep Linkage Blaze's clock splash on that method.
  Its implementation may delegate to `fill_contiguous` with the full-screen
  rectangle, but callers should not need to construct that rectangle.
- Implement and test the smallest callback-based tiled workflow that satisfies
  the hierarchy above. Confirm in Rust that its closure and lending-frame
  lifetimes remain straightforward for application callers.
- Add the canonical four-strategy documentation comparison and framebuffer
  equivalence tests specified above. Review the four call sites together for
  clarity, distinct purpose, coordinate consistency, and honest memory costs.
- Migrate Linkage Blaze's shared clock, skeleton-clock, ballet, and armatron
  implementations plus its ESP, RP, and WASM example crates to the resulting
  API. Prefer changes in shared example code over repeated generated or
  platform-specific edits where the repository structure permits.
- Keep the approved Phase 4B reductions: `Tiles`, `CydDisplay::tiles`,
  `RectanglePixels`, and `flush_at` are not application-facing APIs. Do not
  add another redundant way to draw.
- Update the canonical frame, tiling, and streaming documentation so their
  relative prominence matches the intended application hierarchy.
- Review this API phase before proceeding to the final audit.

### 5. Final Audit

- Re-inventory every rendered CYD page and method section against this spec.
- Fix any remaining visibility, naming, parity, doctest, or link failure.
- Run the complete validation below, with `cargo check-all` last.

#### Phase 5 Correction: Rendered CYD Documentation

Keep the retained application-facing API shape unchanged while correcting the
remaining behavior and rendered-documentation problems found during the final
audit:

- Make the public touch-coordinate contract match the display contract:
  `CydTouch::read` returns calibrated `TouchEvent::Down` and `TouchEvent::Move`
  points in the configured logical display orientation. Their coordinate bounds
  must match `CydDisplay::screen_size()`. Applications must not call
  `Orientation::map_landscape_point` on ordinary touch events.
- Keep calibration data in the panel's fixed 320×240 landscape coordinate
  system. Run the interactive calibration and its verification UI in
  `Orientation::Landscape`; after calibration, apply the requested runtime
  orientation before returning the complete device. A saved landscape
  calibration remains reusable in every runtime orientation and its persisted
  representation must remain compatible.
- Store or otherwise carry the configured orientation at the calibrated-touch
  implementation boundary. ESP and RP must apply
  `Orientation::map_landscape_point` exactly once after raw-to-landscape
  calibration and before constructing an application-facing `TouchEvent`.
  Memory and WASM must satisfy the same public output contract without remapping
  source events that are already in logical display coordinates. Document and
  test each source boundary. If runtime orientation can change, update display
  and touch together so they cannot disagree.
- Audit every caller that currently invokes `map_landscape_point` on a
  `TouchEvent`, including the shared DNS Tester and its tests, and remove the
  now-redundant mapping. Retain `map_landscape_point` for calibration/backend
  work and direct coordinate conversion, not as a required application step.
- Put one focused, compilable `rust,no_run` calibrated-touch example on the
  primary touch type, and link directly to that example from both `CydTouch`
  and `TouchEvent`. The complete-device loop may remain as broader supporting
  coverage, but it is not the focused touch example. The example should use
  `?` to propagate read errors and consume the already-oriented event directly.
- Make each public `cyd`, `tiling`, and `touch` module page contain either a
  compilable example or a direct, working link to the primary type's compiled
  example. The `tiling` page must visibly direct readers to the canonical
  `CydDisplay::for_each_tile` tiled draw loop; prose that merely names the
  method without landing on its example is insufficient.
- Give the core, ESP, and RP top-level `cyd` pages a clear start-here path to
  the complete-device constructor and the device-agnostic `Cyd`, `CydDisplay`,
  and `CydTouch` traits. Keep the ESP and RP introductions parallel where their
  hardware permits.
- Make intra-doc links survive the core module's re-export through the ESP and
  RP crates. Do not use a relative link such as `super::backend` on an
  application-facing page when that target is not re-exported there. Mention
  the backend seam in plain prose or use a link that resolves in every rendered
  location; do not present it as an application API.
- Present the four drawing strategies together in one discoverable place so a
  caller can compare memory use, coordinate handling, and whether drawing must
  be replayed. Keep the framebuffer-equivalence test as the behavioral check;
  this is a documentation correction, not a new drawing API.

The `tiling` and `touch` modules remain public. They are not implementation-only
namespaces: `tiling::TileGrid` and its rectangle-sizing helpers configure the
public `CydDisplay::for_each_tile` workflow, while `touch::TouchEvent` is the
public result of `CydTouch::read`. These namespaces keep display layout and
touch input distinct and give their related public types discoverable homes.
The internal `Tiles` iterator, calibration state machine, raw-point types, and
backend implementation details remain private or confined to the deliberately
narrow `cyd::backend` seam.

This correction is complete only when all of the following are true:

- The rendered core, ESP, and RP `cyd`, `cyd::tiling`, and `cyd::touch` pages
  contain no unresolved bracketed links or misleading links to unavailable
  re-exports.
- All three public module pages provide the direct example coverage described
  above, and every promised link lands on the stated example.
- `CydTouch` and `TouchEvent` both lead directly to the focused calibrated-touch
  example, whose coordinate handling agrees with their prose.
- Tests cover all four orientations and prove that calibrated corner and
  representative interior points returned by `CydTouch::read` lie in the same
  logical coordinate system and bounds as the display. Tests also guard against
  double mapping in migrated consumers.
- Interactive calibration and verification are performed in landscape, while
  the complete device returned to the application uses its requested runtime
  orientation. Existing saved calibration data still loads successfully.
- The ESP and RP rendered pages describe the same shared API contract and differ
  only where platform construction or hardware requires it.
- The existing application public/private boundary remains unchanged; reshape
  the narrow backend seam only if required to carry orientation cleanly. No
  compatibility aliases, lint suppressions, or additional drawing APIs are
  introduced.
- `just update-docs-core`, `just update-docs-esp`, `just update-docs-rp`, and
  `cargo check-all` pass, with `cargo check-all` run last.

### 6. Pass 2: Rendered New-User and Public API Review

This pass follows the rendered ESP `cyd` documentation as a competent Rust and
embedded programmer who has not used Device Envoy before. It is deliberately
separate from the mechanical link and doctest audit above: a compiled example
can still be unusable when essential context is hidden, and a public item can
still be confusing when every link resolves.

Do not restore APIs removed in Phase 4, and do not remove a convenience merely
because another lower-level operation can express the same behavior. Prefer the
path that directly expresses ordinary application intent. Apply shared fixes to
core first and verify that the ESP and RP re-exports render the same contract.

#### Pass 1 Reader Path and Breaks

The natural reader path from the rendered ESP module was:

1. `cyd` module: learned that `CydEsp` is the complete device and
   `CydDisplayEsp` is display-only, but was not told how to choose among
   `CydEsp`, `CydEspOneSpi`, and `CydDisplayEsp`.
2. `CydEsp`: learned that construction owns calibration and requires
   `CydStaticEsp`, but the rendered `CydEsp::new` example uses an undefined
   `touch_spi`, so the first complete-device example cannot be copied or
   understood from visible documentation.
3. `CydStaticEsp`: learned that the application chooses buffer capacity, but
   not how full-frame, regional, or tiled drawing determines the required
   `PIXEL_COUNT`, nor what happens when a frame exceeds it.
4. `CydDisplayEsp`: learned that a standalone display is possible, but its
   example allocates `CydStaticEsp<0>` without explaining that choice or what
   drawing operations remain usable with no frame buffer.
5. `CydDisplay`: found the intended frame, tile, and streaming hierarchy, but
   the published page says there are three strategies and then discusses four
   paths; the four-way comparison exists only as an unlinked host test rather
   than a visible published example.
6. `Cyd` and `CydFrame`: learned the generic device loop and asynchronous frame
   boundary, but then encountered both the generic asynchronous
   `CydFrame::flush` and the ESP concrete synchronous `CydFrameEsp::flush`
   without an explanation of which one application code should call.
7. `CydTouch` and `TouchEvent`: learned that returned points are already in the
   logical oriented display coordinates and must not be mapped again.
8. `Orientation`: encountered a direct contradiction: the
   `map_landscape_point` page says a calibrated `TouchEvent` must be converted
   exactly once. At this point a reader cannot know which touch contract is
   correct.
9. `tiling` and `TileGrid`: learned the replayable low-memory workflow, but the
   sizing example is not connected to `CydStaticEsp` and uses discarded values
   instead of demonstrating a real static buffer decision.
10. `fill_contiguous`, `fill_contiguous_full`, and `draw_items`: learned that
    streaming is advanced, but the regional method links to an example that
    calls only the full-screen method, while the immediate-operations example
    calls `draw_items` with an empty item list and never explains
    `PIXEL_SOURCE_COUNT`.
11. `CydEspOneSpi`: finally learned the shared-bus alternative, but only after
    returning to the module inventory; its documentation leads with bus
    implementation details instead of when a user should choose it.

The intended reader path should instead be explicit on the module page:

`cyd` → choose complete/two-SPI, complete/one-SPI, or display-only construction
→ choose buffer capacity from the drawing workflow → construct → use
`Cyd::parts`/`Cyd::display`/`Cyd::touch` → use full or regional frames by
default → choose tiling when RAM requires it → choose contiguous streaming only
when already producing a raster.

#### Pass 2 Required Corrections

Address these concrete problems in severity order.

1. **Make the touch-coordinate contract consistent.**
   `CydTouch::read` and `TouchEvent` are authoritative: application events are
   already calibrated and mapped into the configured logical display
   orientation. Rewrite `Orientation::map_landscape_point` and its example to
   describe direct conversion of fixed-landscape panel, calibration, or asset
   coordinates. State explicitly that applications must not apply it to a
   `TouchEvent`. Search every rendered core, ESP, and RP page for the obsolete
   instruction.

2. **Add a real start-here constructor decision.**
   The ESP and RP module introductions must explain when to choose the normal
   complete-device constructor, the one-SPI complete-device constructor, or the
   display-only constructor. Lead each one-SPI page with its user-visible reason
   and tradeoff before explaining bus arbitration. Link to a complete runnable
   board example where one exists.

3. **Make the primary constructor examples visible and usable.**
   Essential values such as ESP `touch_spi` must not exist only in hidden
   doctest scaffolding. A visible example may accept a peripheral as a function
   parameter when no single chip supplies a universal concrete value, but the
   reader must see where every argument comes from. Add concise argument
   documentation to the long two-SPI constructor, including the distinction
   between the two buses, calibration flash, and recalibration button. Keep the
   example compile-checked.

4. **Specify the frame-buffer capacity contract.**
   Document how `CydStaticEsp`/`CydStaticRp` capacity relates to
   `full_frame_mut`, `frame_mut`, and `for_each_tile`, including the exact
   failure behavior when a requested frame or tile exceeds capacity. Provide
   meaningful compile-checked storage examples for:
   - `SCREEN_PIXELS` full-frame storage;
   - the largest of fixed regional rectangles; and
   - `TileGrid::max_tile_pixel_count` tiled storage.
   Explain any intentional zero-sized display storage and which immediate or
   streaming operations remain usable with it; otherwise stop using zero in the
   primary `CydDisplayEsp::new`/`CydDisplayRp::new` examples.

5. **Publish the drawing decision model, not only its host test.**
   Describe three mechanisms and four common workflows consistently: full
   frame, regional frame, tiled replay, and contiguous streaming. Put their
   short call shapes, coordinate space, replay requirement, and buffer cost on
   the published `CydDisplay` page. Link the host framebuffer-equivalence test
   only as supplemental evidence. Use “logical display coordinates,”
   “frame-local coordinates,” and “fixed landscape panel coordinates”
   consistently; replace ambiguous “physical-screen coordinates.”

6. **Correct weak or misleading drawing examples.**
   - Point `fill_contiguous` to an example that actually calls regional
     `fill_contiguous`, and specify the required pixel count and behavior for
     short or long iterators for both contiguous methods.
   - Make `draw_items` show at least one real `DrawItem` and explain what
     `PIXEL_SOURCE_COUNT` counts and how callers choose it.
   - Replace discarded `TileGrid` sizing values with assertions or a real
     `CydStaticEsp`/`CydStaticRp` capacity calculation.
   - Make the generic `Cyd` example respond meaningfully to a touch or draw a
     visible item rather than only binding underscore-prefixed values.

7. **Explain the concrete/generic frame split.**
   On `CydFrameEsp` and `CydFrameRp`, explain that the inherent concrete
   `flush` is synchronous while platform-neutral `CydFrame::flush` is awaited,
   and show the intended call form for each. Align `write_text` wording around
   frame-local `(0, 0)` and make creation of the concrete frame discoverable
   from its example rather than presenting a frame parameter with no origin.
   Retain useful inherent conveniences; do not remove them merely because the
   generic trait has related methods.

8. **Finish the public-item explanations.**
   Add useful one-line descriptions for `Cyd::Display`, `Cyd::Touch`, all
   `TouchEvent` variants and point fields, all `Orientation` variants, and the
   public `TileGrid` fields or their replacements. Application-facing `touch`
   documentation should lead with reading events; move platform-author backend
   discussion to `cyd::backend`.

#### Public Surface Decisions Requiring Audit

- `TileGrid::top_left` and `TileGrid::size` are publicly mutable while the
  constructor validates the grid counts. Audit whether mutation can bypass or
  invalidate those invariants. Prefer private fields plus `top_left`, `size`,
  or `rectangle` getters unless a demonstrated application needs struct-literal
  construction or mutation. Also evaluate `TileGrid::new(rectangle, columns,
  rows)` as the clearer constructor shape.
- Audit application use of `CydFrame::tile_top_left`. Its rendered semantics are
  translation plumbing, and ordinary tiled drawing is already translated by
  `for_each_tile`. Move it to the backend seam or remove it if no supported
  application needs to inspect it; otherwise document the concrete application
  use that justifies it.
- Resolve the asymmetry between `Cyd::display()` and the absence of
  `Cyd::touch()`. A `touch()` convenience directly expresses common intent and
  is preferable to forcing generic callers to borrow and discard the display
  half. Review the public `CydEsp`/`CydRp` component fields at the same time and
  establish one canonical access story.
- Keep `DisplayBackend` public because platform implementations are separate
  crates, but make every description visible through `CydDisplayEsp` and
  `CydDisplayRp` say immediately that it is a platform-author seam. Strengthen
  the backend example so it explicitly names or implements `DisplayBackend`;
  merely using `D: CydDisplay` does not demonstrate the item.
- Review the long primary constructors against the project's 90/10 goal. The
  common path currently requires explicit SPI policy, colors, and font even
  though defaults exist. Before changing signatures, propose one direct,
  non-builder constructor hierarchy that makes the usual device easy while
  preserving an explicit advanced configuration path. Review that proposal
  before implementation.

Do not remove `CydDisplayEsp`, `CydTouchEsp`, `CydFrameEsp`, their RP peers,
`Error`, `Orientation`, the two default constants, `Cyd`, `CydDisplay`,
`CydTouch`, `TouchEvent`, `TileGrid`, or the rectangle-sizing helpers merely to
shrink the sidebar. They all have demonstrated signature, construction,
drawing, or storage roles. The questions above concern invariants, hierarchy,
and user intent rather than raw item count.

#### Pass 2 Acceptance

- Repeat the rendered new-user walkthrough without implementation source. The
  reader must be able to choose a constructor, choose valid storage, construct
  a device, draw and flush a normal frame, and read oriented touch without
  guessing or encountering contradictory instructions.
- Re-run the public-item checklist against every canonical non-redirect core,
  ESP, and RP CYD page. Record specific coverage for every retained method and
  variant, not only a page count.
- Add focused tests for any `TileGrid` visibility/invariant change and for the
  documented insufficient-buffer and contiguous-iterator behavior.
- Keep the rendered unresolved-link scan at zero matches and run all published
  feature/target doctest combinations.
- Migrate Device Envoy examples, Linkage Blaze, and the ESP/RP starters for any
  accepted API change. Run `just update-docs-core`, `just update-docs-esp`,
  `just update-docs-rp`, both workspaces' focused checks, `git diff --check`, and
  Device Envoy `cargo check-all` last. Stop for review before implementing a
  constructor redesign or removing a questioned public item.

## Validation

- Search examples and downstream starter projects before removing each item.
- Treat the sibling Linkage Blaze workspace as a required CYD consumer: search
  and migrate its shared examples and ESP/RP/WASM wrappers, then run its focused
  checks during the drawing phase and its `cargo check-all` before release.
- Build core, RP, and ESP rustdoc and reject unresolved intra-doc links.
- Run doctests with the same feature combinations used to publish the retained
  documentation. A doctest hidden behind a feature absent from docs.rs does not
  count as coverage.
- Inspect every retained rendered CYD page and method section against the
  doctest checklist; verify direct links land on the promised example.
- Inspect the rendered ESP, RP, and core CYD module introductions, including
  external cross-crate links; source-level intra-doc-link checks alone do not
  prove that docs.rs selected a compatible published dependency version.
- Run `cargo check-all` after implementing the cleanup.
