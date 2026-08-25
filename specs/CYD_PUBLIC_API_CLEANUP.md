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
- Make `CydTouchUncalibrated`, `RawPoint`, and `RawTouchEvent` private along
  with the platform `CydTouchUncalibratedEsp` and `CydTouchUncalibratedRp`
  types. They currently exist to connect the platform implementations to the
  shared calibration driver, not to support application code.
- Make `CalibrationConfig`, `CalibrationFlow`, `CalibrationValidation`,
  `CalibrationCorner`, `EnsureCalibrationOutcome`,
  `EnsureCalibrationSettings`, the calibration `Error`/`ErrorKind`, and all
  calibration tuning constants, geometry helpers, drawing helpers, solve and
  validation functions, and `ensure_calibration*` functions private.
- Preserve the serialized calibration data and automatic flash-backed behavior
  behind the complete-device constructors; making its Rust type private must
  not silently invalidate already stored calibration without an intentional
  format/version decision.
- Resolve the core-to-platform crate seam instead of treating it as a public
  API requirement. Relocate orchestration or otherwise reshape ownership so
  ESP and RP do not need to import undocumented core implementation items.
  Do not use `#[doc(hidden)]` as a substitute for that refactor.

The workspace examples and the ESP/RP starter projects currently use only the
complete-device constructors and calibrated touch reads. They do not name any
of the calibration flow, configuration, validation, geometry, drawing,
settings, outcome, or error items above. Re-run that downstream audit during
implementation in case usage changes.

## Consolidate Errors

Fold the platform-specific display-init, display-flush, and touch-init label
enums into each platform module's `cyd::Error`. The current nested enums add
names without retaining underlying source errors, and the crate-wide error
type wraps them again. Keep enough variant detail to distinguish configuration,
SPI-device creation, panel initialization, frame flushing, and orientation
failures.

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

The calibration `ErrorKind` is not a useful public error in any form: the
calibration subsystem is private, and its failures must be translated at the
constructor boundary into the platform's `cyd::Error` while preserving the
underlying diagnostic.

## Documentation Rules

- Audit the rendered public items recursively on every ESP, RP, and core CYD
  page and subpage. Each item must have a demonstrated application use or be
  removed from the public surface; internal cross-crate convenience alone is
  not sufficient justification.
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
  `CydTouchUncalibrated` as bracketed code; redesign the calibration boundary
  so every type named by public documentation and signatures is either a real
  documented API or no longer public.
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
  touch-read example. Items removed with the private calibration surface need
  no doctests.
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

## Validation

- Search examples and downstream starter projects before removing each item.
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
