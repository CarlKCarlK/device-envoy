<!-- todo0 consider deleting this spec once the work below is implemented, verified on real hardware, and released. -->

## Persistent flash block allocation

The DNS tester reserves three contiguous flash blocks in this order on both
ESP and RP:

1. Wi-Fi credentials and connection state
2. Touch calibration
3. DNS-app display orientation

Each setting remains independently saved and cleared. Changing this allocation
order changes the meaning of existing raw blocks, so devices using an earlier
build should have their settings cleared or be fully reprovisioned before
testing the new build.

# DNS tester example + buffer-free calibration UI

## Goals and reset UX

The example has three purposes:

1. Verify that Wi-Fi and touch can operate together for long periods.
2. Make saved Wi-Fi credentials resettable while the device is unattended.
3. Make saved touch calibration resettable when the display or touch wiring changes.

The reset controls should be consistent on ESP and RP and should not depend on
long-press timing. A normal debounced press is enough because the action is
phase-specific:

| Phase | On-screen control | Physical button |
| --- | --- | --- |
| Calibration | Calibration targets | Restart the current calibration flow |
| Wi-Fi connecting | — | Clear Wi-Fi state and reboot into the captive portal |
| DNS tester running | `CAL` and `WiFi` buttons | Recalibrate as a backup |

The running DNS screen should provide small `CAL` and `WiFi` buttons in a
reserved control area. A touch on either button is a control action and must
not also count as a DNS tap; all other touches continue to trigger one DNS
query.

The `CAL` action clears the calibration flash block and reboots so startup
enters the shared calibration flow. The physical button provides the same
calibration reset as a recovery path when the touchscreen is unusable.

The calibration flow itself is always displayed in landscape, regardless of
the saved DNS-app orientation. If calibration is missing or has been cleared,
the DNS app must initialize the display in landscape, complete calibration,
and reboot before applying the saved user orientation. This keeps the shared
calibration targets inside their fixed landscape coordinate system.

The `WiFi` action clears saved Wi-Fi credentials, selects captive-portal
startup mode, and reboots once the DNS loop owns touch input. The physical
button must perform the same Wi-Fi reset while Wi-Fi is connecting, when the
DNS loop is not yet reading touch input. ESP and RP should expose the same
behavior, even though their Wi-Fi backends have different connection
implementations.

Every reset path must wait for the initiating input to be released before
entering the next phase or rebooting. This prevents one held button or touch
from being interpreted again after reboot, for example as both a calibration
reset and a Wi-Fi reset.

The running DNS screen should also provide an orientation-cycle button. Each
activation advances through `Landscape`, `Portrait`, `LandscapeInverted`, and
`PortraitInverted`, saves the selected orientation, and reboots. On the next
boot the DNS display layout initializes in the saved orientation. Orientation
changes must use the same release gating as the other reset actions.

Orientation is currently selected at display construction time, so rebooting
is intentional rather than an in-place display mutation. The orientation
setting should have its own persistent flash record, separate from Wi-Fi
credentials and touch calibration. Orientation is a DNS-app presentation
setting; changing it does not invalidate or rerun the shared touch calibration.
The DNS app must instead keep its status text, control buttons, and hit testing
correct for the selected display orientation. It must transform the calibrated
landscape touch point into the selected DNS-app screen coordinates before hit
testing controls or using it as an app-level point.

## What we did

**New example: touch-triggered Wi-Fi/DNS reliability tester ("dns_tester")**

- Tap anywhere on a CYD touchscreen to fire one DNS query (`example.com`) and
  update a single status line with running tap/success/fail counts and last
  round-trip latency. Meant to run for hours unattended on hardware with no
  reachable physical reset button, exercising touch + Wi-Fi together.
- ESP: `crates/device-envoy-examples-esp/examples/templates/dns_tester.rs.j2`, generated
  for all 13 chip/board profiles via the examples-ESP xtask's
  `generate-board-examples` command.
  Real code on the 4 boards with dual SPI + Wi-Fi + large stack (esp32,
  esp32s3 ×3); placeholder-stubbed elsewhere.
- RP: hand-written `crates/device-envoy-examples-rp/examples/dns_tester.rs` (no
  templating on this side). Builds for Pico1 W and Pico2 W (`w`/`2w`);
  compiles to nothing on `1`/`2` via `#![cfg(feature = "wifi")]`, matching
  existing RP examples.

**Core fix: buffer-free calibration UI (`device-envoy-core`)**

- `ensure_calibration()` used to draw its target/dot UI via
  `display.full_frame_mut()`, which requires a static workspace ≥ the full
  screen (320×240 = 76,800px / 150KB) whenever calibration hasn't been saved
  to flash yet — regardless of what the calling example itself draws.
  Combined with Wi-Fi's own heap, this overflowed plain ESP32's DRAM at link
  time (`stack.x: cannot move location counter backwards`), reproduced
  directly with `cargo +esp build --release`.
- Rewrote `draw_calibration_screen`/`draw_message_screen`
  (`crates/device-envoy-core/src/cyd/touch/driver.rs`) to draw targets/dots
  via `CydDisplay::draw_items()` (the `DrawItem` → `ContiguousPixels` →
  `fill_contiguous` streaming path, zero static buffer, already proven by
  linkage-blaze's clock-hands rendering) and moved the instruction/
  confirmation text into one small buffered banner,
  `CALIBRATION_TEXT_RECTANGLE` (320×20px). Exactly one `flush()` per redraw is
  preserved, since the test harness's frame-pacing and scripted-touch-event
  advancement is wired to `CydFrame::flush()`, not to `fill_contiguous`.
- Added `pub const CALIBRATION_MIN_PIXEL_COUNT` (in
  `crates/device-envoy-core/src/cyd/touch/calibration.rs`) so example authors
  have a real number to size their static buffer against instead of guessing.
  Both `dns_tester` examples reference it directly.

**Verification**

- Core: all 71 tests + 46 doctests pass (one assertion updated — a literal
  "last flush was the full screen" check now correctly checks the small text
  banner instead).
- Real `cargo +esp build --release` (not just `cargo check`, which doesn't
  fully link) on classic ESP32 (`esp32/generic`): clean, no warnings.
- the examples-ESP xtask's board-aware `check-examples-all-processors` command
  across all 9 ESP chip families: full run
  completed, zero failures.
- RP: `cargo check-all` (all migrated examples) passes; real
  `cargo build --release` on Pico1 W (RP2040, tightest RAM at 256KB) links
  clean too — previously untested territory for touch calibration + Wi-Fi.
- WASM/host feature build: clean.
- `linkage-blaze-core` build + tests: unaffected, all pass.

## What we might want to do additionally

- **Flash and visually eyeball the redesigned calibration UI on real
  hardware.** All the above is compile/link/unit-test verification; nobody
  has watched the buffer-free target/dot rendering actually draw on a
  physical CYD panel yet. The touch-script-driven memory tests check exact
  pixel colors at exact coordinates and all pass, which is a strong signal,
  but it's not the same as seeing it.
- **Flash `dns_tester` on RP hardware** (Pico W / Pico 2 W) — only
  build/link-verified so far, same as the ESP side originally was before the
  memory bugs surfaced.
- Consider a dedicated golden-image (PNG) test for the calibration target
  rendering, similar to `clock_renders_expected_frame` in linkage-blaze, to
  lock in visual correctness going forward rather than relying solely on the
  touch-script pixel-color assertions.
- Implement the reset UX described above. In particular, the Wi-Fi reset must
  remain available after the connection phase through the on-screen `WiFi`
  button, while the physical button remains the calibration backup once the
  DNS tester is running.
- No non-touch fallback exists for boards that support Wi-Fi but not dual
  SPI (e.g. esp32c3/c6 generic) — they currently just get a placeholder stub
  for `dns_tester`. A timer-driven (no-touch) variant could exercise Wi-Fi
  reliability on those boards too, if that's wanted.
