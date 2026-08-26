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
   as an equal starting point.

Audit the existing methods accordingly:

- Move `frame_mut_with_tile_top_left` out of the normal application-facing
  story; make it backend-only if cross-crate implementation requirements
  prevent making it private.
- Prefer `for_each_tile` over requiring applications to operate the `Tiles`
  lending iterator directly. Remove or privatize `Tiles` and `tiles` if the
  callback helper covers supported downstream uses.
- Audit `fill_contiguous_full`, `flush_at`, and `draw_items` for redundancy or
  backend-only use. Retain them publicly only when a demonstrated application
  use justifies them.
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
- Keep one canonical compiled `rust,no_run` tiling example, preferably on
  `CydDisplay::tiles`. Make the `tiling` module, `TileGrid`, `Tiles`, their
  public methods, and the public rectangle-size helpers either contain a
  focused compiled example or link directly to that canonical example.
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
problem affects the `CydDisplay::screen_size`,
`CydDisplay::frame_mut_with_tile_top_left`, and `CydDisplay::frame_mut`
examples. Add unconditional, platform-neutral canonical examples for the
public traits. Host-only framebuffer examples and preview images may remain as
supplemental documentation, but public documentation must not depend on them.

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
- `display`, `CydFrame`, and `RectanglePixels`: link to canonical examples that
  actually exercise their primary types and every retained method.
- `DrawItem`/`Image565View`, `Orientation`, and the retained compile-time TGA
  image/mask types, functions, and macro: provide one focused canonical example
  per coherent family, with every family member and method linked directly to
  it.
- `tiling`, `TileGrid`, `Tiles`, `rectangle_pixel_count`, and
  `max_rectangle_pixel_count`: use `CydDisplay::tiles` as the canonical draw
  loop and add a focused grid/buffer-sizing example where needed. Link every
  retained getter and `Tiles::next` directly to the applicable example.
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

The published pages currently fail this audit broadly: the primary `Cyd`
example is absent, links from `CydTouch`, `TouchEvent`, `CydFrame`, and several
display methods therefore lead to no compiled example, `Orientation` has none,
the image/TGA family has none, and the tiling getters and helpers do not all
link directly to the existing `CydDisplay::tiles` doctest. The ESP/RP concrete
constructors and retained component methods likewise lack complete direct
coverage. Treat this list as the starting audit, then re-inventory the rendered
pages after visibility changes so newly retained items cannot escape it.

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

- Audit downstream and example use of `frame_mut_with_tile_top_left`, `Tiles`,
  `tiles`, `fill_contiguous_full`, `flush_at`, and `draw_items` before changing
  their visibility or removing them. Include the sibling Linkage Blaze
  workspace and classify its shared and platform-specific CYD examples.
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
- Remove or privatize superseded peer-level paths instead of adding another
  redundant way to draw.
- Update the canonical frame, tiling, and streaming documentation so their
  relative prominence matches the intended application hierarchy.
- Review this API phase before proceeding to the final audit.

### 5. Final Audit

- Re-inventory every rendered CYD page and method section against this spec.
- Fix any remaining visibility, naming, parity, doctest, or link failure.
- Run the complete validation below, with `cargo check-all` last.

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
