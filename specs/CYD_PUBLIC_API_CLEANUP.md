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

## Remove From the Public Surface

- Make `PixelBuffer` private; it is storage hidden behind `CydStaticEsp` and
  `CydStaticRp`.
- Delete `RegionBuffer` if a final usage audit confirms that no supported
  application path needs it.
- Remove or privatize `CydFrameEsp::view_mut` and `CydFrameRp::view_mut` if
  `DrawTarget`, `fill`, and `raw_pixels_mut` cover the supported use cases;
  then make `RegionView` private.
- Make `TOUCH_SPI_HZ` private because touch constructors do not expose clock
  selection as policy.
- Remove the uncalibrated platform touch types from the application-facing
  surface. Reshape the shared calibration traits and errors as needed rather
  than hiding public types from rustdoc.

## Consolidate Errors

Fold the platform-specific display-init, display-flush, and touch-init label
enums into each platform module's `cyd::Error`. The current nested enums add
names without retaining underlying source errors, and the crate-wide error
type wraps them again. Keep enough variant detail to distinguish configuration,
SPI-device creation, panel initialization, frame flushing, and orientation
failures.

## Documentation Rules

- Do not use `#[doc(hidden)]` to conceal ordinary implementation details; make
  them private or redesign the public signature that leaks them.
- Keep `CydTouch` public and visible because applications import it to call
  calibrated touch operations such as `read`.
- Keep module introductions and public surfaces parallel between ESP and RP.
- Use the multiplication sign in prose dimensions, for example `320×240`.

## Validation

- Search examples and downstream starter projects before removing each item.
- Build core, RP, and ESP rustdoc and reject unresolved intra-doc links.
- Run `cargo check-all` after implementing the cleanup.
