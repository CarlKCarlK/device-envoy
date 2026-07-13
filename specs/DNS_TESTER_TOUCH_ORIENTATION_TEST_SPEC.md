<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# DNS Tester Touch and Orientation Confidence

## Objective

Make touch behavior predictable and well tested in all four CYD orientations
across the shared runtime, `CydMemory`, ESP, RP, and WASM.

The implementation should catch coordinate rotation, inversion, swapped-axis,
off-by-one, double-mapping, and incorrect hit-region bugs before hardware
testing. Final hardware validation will use the existing DNS Tester application.

## Non-goals

- Do not add a diagnostic screen, calibration screen, crosshair overlay, debug
  panel, or other new hardware UI.
- Do not add platform-specific DNS Tester touch policy.
- Do not duplicate orientation mapping in ESP, RP, WASM, and the shared runtime.
- Do not change the visual layout unless a test proves that a layout rectangle
  is incorrect.
- Do not require hardware in automated CI.

## Coordinate contract

Use three explicitly documented coordinate spaces:

```text
touch-controller coordinates
    -> calibration
fixed landscape-panel coordinates (320x240)
    -> Orientation::map_landscape_point
logical UI coordinates (320x240 or 240x320)
```

`CydTouch::read()` must return calibrated `TouchEvent` points in fixed
landscape-panel coordinates, regardless of the current display orientation.
The shared DNS Tester UI applies `Orientation::map_landscape_point` exactly
once before layout hit testing.

Expected mapping from landscape point `(x, y)` is:

| Orientation | Logical point |
| --- | --- |
| `Landscape` | `(x, y)` |
| `Portrait` | `(y, 319 - x)` |
| `LandscapeInverted` | `(319 - x, 239 - y)` |
| `PortraitInverted` | `(239 - y, x)` |

The implementation must document this contract on `CydTouch::read()`,
`Orientation::map_landscape_point`, and any platform adapter whose behavior is
otherwise ambiguous.

## Phase 1: audit the complete coordinate path

Trace and record the path for each backend:

- ESP touch controller -> calibration -> `CydTouch::read()`;
- RP touch controller -> calibration -> `CydTouch::read()`;
- browser pointer coordinates -> WASM `CydTouch::read()`;
- scripted `TouchEvent` -> `CydMemory` `CydTouch::read()`.

For each path, confirm:

- calibration produces fixed landscape-panel coordinates;
- display orientation does not rotate the point before the shared UI sees it;
- the shared UI performs the only orientation mapping;
- `Cyd::orientation()` returns the exact saved orientation, including the two
  inverted variants;
- no backend infers an inverted orientation from dimensions alone;
- width and height are not confused when converting portrait pointer input.

If a platform currently emits logical UI coordinates, move that conversion to
the appropriate adapter boundary so all platforms satisfy one contract. Do not
add compensating platform cases to DNS Tester.

## Phase 2: exhaustively test orientation math

Add host tests for all four `Orientation` values.

At minimum test:

- all four landscape-panel corners;
- all four edge midpoints;
- the center and several asymmetric interior points;
- points one pixel inside each edge;
- the corresponding logical screen bounds.

Also test these properties:

- every mapped point is inside `orientation.size()`;
- mapping has a test-only inverse and round-trips every representative point;
- preferably, all 76,800 landscape-panel pixels round-trip for every
  orientation;
- `orientation.next()` visits each state once and returns to the starting state
  after four calls;
- orientation width, height, and pixel count agree.

The test-only inverse mapping from logical point `(u, v)` is:

| Orientation | Landscape point |
| --- | --- |
| `Landscape` | `(u, v)` |
| `Portrait` | `(319 - v, u)` |
| `LandscapeInverted` | `(319 - u, 239 - v)` |
| `PortraitInverted` | `(v, 239 - u)` |

Do not add a public inverse API solely for tests unless production adapters
also have a real need for it.

## Phase 3: make `CydMemory` model orientation faithfully

`CydMemory::new_with_orientation` must retain and report the exact supplied
orientation. Screen dimensions alone cannot distinguish `Landscape` from
`LandscapeInverted`, or `Portrait` from `PortraitInverted`.

Add tests that verify:

- all four supplied orientations round-trip through `Cyd::orientation()`;
- `new(size, ...)` preserves its documented dimension-based default behavior;
- splitting and reconstructing through `CydParts` has explicit behavior.

If `CydParts::from_parts` cannot recover inversion because the parts do not
carry it, document and test the chosen non-inverted fallback. Do not silently
claim that inversion was preserved.

Scripted calibrated touch events in `CydMemory` must follow the same fixed
landscape-panel contract as hardware implementations.

## Phase 4: test the complete DNS Tester interaction matrix

Use `CydMemory` to run the real shared `dns_tester` loop. For every orientation,
inject landscape-panel points that map to each logical region.

### Control centers

Test the center of each visible control:

- Calibration returns `Exit::Calibrate`;
- Wi-Fi returns `Exit::ResetWifi`;
- Orientation returns `Exit::Reorientate(orientation.next())`.

### Ordinary dashboard touch

Tap at least two points outside all control rectangles. Assert that each
touch-down calls `Dns::lookup()` exactly once and does not produce a platform
exit by itself.

Use a counting DNS test double rather than inferring lookup calls from rendered
text.

### Event kinds and consumption

Verify:

- `TouchEvent::Down` triggers an action;
- `TouchEvent::Move` does not trigger an action;
- `TouchEvent::Up` does not trigger an action;
- one down event cannot activate two adjacent controls;
- an event is not replayed on later frames;
- a DNS lookup is not repeated while no new down event is available.

### Hit-region boundaries

For every control in both layouts, test:

- top-left included;
- final included pixel at the bottom-right edge;
- first pixel immediately left, right, above, and below excluded unless that
  pixel deliberately belongs to the neighboring control;
- shared boundaries between adjacent controls route to exactly one control;
- all control rectangles fit inside their logical screen;
- control rectangles do not overlap.

Run these checks through the oriented touch path, not only by calling
`Layout::control_at` directly.

### Rendering regression

Retain the four existing golden PNG checks. The test must ensure that
`CydMemory` reports the intended orientation to the shared loop before the
frame is rendered.

Golden images prove visual placement, while interaction tests prove touch
placement. Neither substitutes for the other.

## Phase 5: platform adapter tests

Keep platform tests focused on the adapter boundary rather than duplicating the
DNS Tester interaction matrix.

### ESP and RP

Where possible, extract controller/calibration coordinate conversion into pure
functions and test known raw or calibrated vectors. Confirm that the output of
`CydTouch::read()` is fixed landscape-panel coordinates in every display
orientation.

At minimum include asymmetric points so swapped axes cannot accidentally pass,
for example points near `(23, 71)` and `(287, 204)`.

Confirm that changing display orientation does not also change calibrated touch
output before it reaches the shared runtime.

### WASM

Test browser pointer conversion for all four orientations, including canvas
scaling and portrait dimensions. Start from browser/canvas coordinates and
assert the fixed landscape-panel point supplied to the shared runtime.

Test at least:

- canvas origin and opposite corner;
- center;
- asymmetric interior points;
- non-1:1 CSS-to-canvas scaling;
- points at the final valid pixel.

Avoid testing only square or central points because they hide axis errors.

## Phase 6: build and CI coverage

The completed change must run:

- orientation and `CydMemory` unit tests;
- DNS Tester memory interaction tests;
- four DNS Tester golden-image tests;
- WASM tests;
- RP example checks;
- ESP example checks;
- generated ESP template consistency checks;
- repository `just check-all` local CI.

Fix the source ESP template first, then regenerate examples. Do not patch only
generated files.

## Existing-application hardware validation

After automated work passes, validate with the current DNS Tester application.
Do not create a new screen for this validation.

For each platform available for testing and each of the four orientations:

1. Tap an ordinary dashboard location and confirm exactly one DNS query runs.
2. Tap Calibration and confirm the calibration flow starts.
3. Tap Wi-Fi and confirm Wi-Fi reset/setup starts.
4. Tap Orientation and confirm the next orientation is persisted and shown
   after restart.
5. Confirm each visible button responds near its center and does not respond
   when tapping clearly outside it.
6. Cycle through all four orientations and confirm the fifth orientation action
   returns to Landscape.

If a hardware failure remains, record:

- platform and board;
- saved display orientation;
- visible button tapped;
- action actually observed;
- whether the error resembles 90-degree rotation, 180-degree inversion,
  swapped axes, or offset/scaling.

Use that observation to add a failing automated adapter test before changing
the mapping.

## Acceptance criteria

The work is complete when:

- one documented coordinate contract applies to Memory, ESP, RP, and WASM;
- orientation mapping is applied exactly once;
- exhaustive orientation math tests pass;
- `CydMemory` faithfully reports all four orientations;
- every DNS Tester control and ordinary touch behavior is tested in every
  orientation;
- platform adapters have focused coordinate tests;
- all platform examples build and generated files are current;
- the existing DNS Tester app passes the hardware checklist;
- no new diagnostic or hardware-test screen was introduced.
