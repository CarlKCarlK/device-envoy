# Changelog

## 0.2.0 (unreleased)

Every declaration macro is now defined by a
[`macro-schema`](https://crates.io/crates/macro-schema) schema instead of a
hand-written `macro_rules!` parser. The macros report unknown, duplicate,
missing, and wrong-kind fields at the caller's tokens; their syntax and field
tables in the docs are generated from the schema; and every declared item gets
a generated configuration table. This release is a clean break: the 0.1 syntax
is not accepted.

Syntax changes:

- Shared group fields go inside the group's braces, and members are written
  `Name { ... }` rather than `Name: { ... }`. This affects `led_strips!`,
  `irs!`, `ir_keplers!`, `ir_mappings!`, `i2cs!`, and `lcd_text!`:

  ```rust,ignore
  // 0.1
  led_strips! {
      pio: PIO0,
      pub LedStrips {
          Strip0: { pin: PIN_2, len: 8, max_current: Current::Milliamps(250) },
      }
  }

  // 0.2
  led_strips! {
      pub LedStrips {
          pio: PIO0,

          Strip0 { pin: PIN_2, len: 8, max_current: Current::Milliamps(250) },
      }
  }
  ```

- Fields may be written in any order, a trailing comma is always optional, and
  every declaration and member accepts attributes.
- Every declaration accepts a visibility. Members inherit their group's
  visibility. ESP `servo!`, ESP `servo_player!`, and the IR groups were always
  `pub`; they now follow the visibility you write.
- RP `servo!` declares a named type, like ESP `servo!`. The PWM slice is passed
  to `new`, and a trait bound checks that it is the pin's slice. The `slice`,
  `channel`, `odd`, and `even` fields are gone:

  ```rust,ignore
  // 0.1
  let mut servo = servo! { pin: p.PIN_11, slice: p.PWM_SLICE5 };

  // 0.2
  servo! { Servo11 { pin: PIN_11 } }
  let mut servo = Servo11::new(p.PIN_11, p.PWM_SLICE5);
  ```

- `pcm_clip!` accepts only `source_sample_rate_hz`; the `sample_rate_hz` alias
  is gone.
- ESP `led2d!` no longer takes `len`; the LED count comes from `led_layout`, as
  on RP.
- ESP `led_strip!` and `led2d!` name their output engine with a type:
  `engine: Spi` or `engine: Rmt` (from `device_envoy_esp::led_strip`) instead
  of `engine: Engine::Spi`. `Engine` is gone. Naming `Rmt` on a chip without
  an RMT peripheral is a compile error that says so.
- ESP `init_and_start!` takes only the peripherals binding:
  `init_and_start!(p);`. Create shared peripherals with functions, so you
  choose how to handle errors: `let rmt80 = device_envoy_esp::new_rmt80(p.RMT)?;`
  (add `.into_async()` for async devices) and
  `let ledc = device_envoy_esp::new_ledc(p.LEDC);`. The `rmt80:`, `mode:`, and
  `ledc:` options and the `rmt_mode` module are gone, and the macro no longer
  panics if RMT setup fails.
- In `led_strips!`, `max_frames` for a panel is set on the member rather than
  inside its `led2d` block.
- `pio_split!` has been removed.

API changes (RP and ESP now generate the same API):

- `led!` and `led2d!` constructors return `&'static Self`, like every other
  device. Core implements each device trait for `&T`, so code generic over a
  trait (`impl Led2d`, `impl LedStrip`, ...) accepts these handles.
- A panel implements only `Led2d`. Its strip is reached through `strip()`,
  which returns the underlying strip (it implements `LedStrip`). On RP,
  `led_strips!` panel members also return `&'static` panels.
- ESP `servo_player!` matches RP: `new` returns `&'static Self`, and the
  declared type implements `Servo` and `ServoPlayer`.
- Constructor arguments go from most specific to least: the device's own pins,
  then its peripherals or channels, then shared resources, then the spawner.
  Group constructors take each member's arguments first and the shared
  resource last:

  ```rust,ignore
  // 0.1
  let (strip0, strip1) = LedStrips::new(p.PIO0, p.PIN_2, p.DMA_CH0, p.PIN_3, p.DMA_CH1, spawner)?;
  let lcd = Lcd::new(p.I2C0, p.PIN_4, p.PIN_5, spawner)?;
  let servo = Servo11::new(&ledc, p.GPIO11)?; // ESP

  // 0.2
  let (strip0, strip1) = LedStrips::new(p.PIN_2, p.DMA_CH0, p.PIN_3, p.DMA_CH1, p.PIO0, spawner)?;
  let lcd = Lcd::new(p.PIN_4, p.PIN_5, p.I2C0, spawner)?;
  let servo = Servo11::new(p.GPIO11, &ledc)?; // ESP
  ```

  Schema fields are documented in the same order; you may still write them in
  any order.
- RP `led!`, `lcd_text!`, and `i2cs!` accept `impl Into<Peri<'static, _>>`,
  like the other RP macros.
- ESP `button_watch!`'s constructor parameter is named `pin`.
- ESP `led_strip!` and `led2d!` document `engine` as a defaulted field ("RMT
  where the chip has it, otherwise SPI").
- ESP `COMBO_TABLE` and `servo!`'s generated static struct are private.

Behavior changes and fixes:

- RP IR macros default `pio` to `PIO0`, like the LED and audio macros.
- ESP `led2d!` chooses its engine by chip, as `led_strip!` does: RMT where
  available, otherwise SPI. Previously panels defaulted to RMT even on chips
  without it.
- ESP strip `len` and `reset_us` can name constants private to the caller's
  module.
- An ESP RMT LED strip no longer goes permanently dark after one failed
  transmission: the driver keeps its RMT channel, and write failures are logged
  as warnings instead of being dropped.
- LCD text write failures are logged as warnings instead of being dropped, and a
  display whose initialization fails is initialized again on its next frame.
- Generated code no longer requires your crate to depend on `paste`,
  `static_cell`, `heapless`, or `embassy_sync`.
- Every item the macros generate is documented, so a crate that denies
  `missing_docs` builds, and links in generated documentation resolve instead of
  pointing at your own crate.
- Two RP LED groups on one PIO in one module, ESP LEDC channel clashes, and
  duplicate LCD addresses fail to compile with errors that name the rule.

## 0.1.6

- Fixed CYD runtime orientation changes by upgrading `mipidsi`, so display
  drawing uses the current logical bounds after calibration.
- Increased the DNS tester frame buffer to fit the landscape calibration
  banner.
- Updated `embedded-graphics` to 0.8.2.

## 0.1.5

- Added portable CYD framebuffer pixel lookup and aligned pixel access across
  Core, RP, ESP, memory, and WebAssembly implementations with
  `embedded-graphics` behavior.
- Reduced browser-simulator touch latency by coalescing queued pointer events.
- Refined the CYD platform integrations, including shared display reset/enable
  handling and consistent `FlashBlock` value naming.
- Added the `device-envoy-cyd-starter` project and browser demo to the Core,
  RP, and ESP documentation, with a shared cross-platform documentation
  preview command.

## 0.1.4

- Fixed and expanded Core, RP, and ESP docs.rs output for the CYD APIs,
  including host and WebAssembly implementation documentation and embedded
  images.
- Replaced Core's absolute docs.rs self-links with version-local intra-doc
  links and added a release check that rejects unchecked Core self-links.
- Added colorful, clickable Linkage Blaze gallery previews to the Core, RP,
  and ESP CYD documentation.

## 0.1.3

- Refined the pre-adoption CYD display and touch APIs across Core, RP, and ESP,
  including logical coordinates, tiled drawing, and fallible touch reads.
- Expanded CYD image, memory, browser-simulation, and API documentation support
  and removed the experimental-status warnings from the public READMEs.

## 0.1.2

- Fixed the `device-envoy-rp` package so its generated PCM and ADPCM sample
  modules use audio assets included in the crate rather than files from a
  sibling workspace crate.
- This is an RP-only corrective release; `device-envoy-core` and
  `device-envoy-esp` remain at `0.1.1`.

## 0.1.1

- Expanded shared CYD display, touch, calibration, memory, and browser-simulator support.
- Reorganized shared Conway and DNS tester applications across host, WASM, RP, and ESP targets.
- Improved error propagation and diagnostics across the core and platform crates.
- Added audio playlist examples and strengthened audio wiring validation.
- Added pinned MSRV and Clippy CI checks.
- Updated `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to depend on the new platform crate versions.

## 0.1.0

- Bumped published workspace crate versions to `0.1.0`.
- Updated downstream sample projects `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to `0.1.0`.
- Fixed Pico W WiFi hang after `cyw43` 0.6→0.7 migration: `cyw43 0.7` requires the caller to supply the NVRAM blob; the empty slice previously passed is now replaced with the actual Pico W NVRAM constants.
- Fixed silent I2S audio after `embassy-rp` 0.9→0.10 migration: `PioI2sOut::new()` no longer auto-starts the state machine; `.start()` is now called explicitly after construction.
- Added `examples/rfid.rs` for the RP crate: a minimal runnable RFID example using the MFRC522 module over SPI0 (CS=PIN_1, SCK=PIN_2, MOSI=PIN_3, MISO=PIN_4, RST=PIN_5).

## 0.0.6-alpha.1

- Added Conway's Game of Life WebAssembly demo published to GitHub Pages at `carlkcarlk.github.io/device-envoy/conway/`.
- Updated downstream sample projects `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to `0.0.6-alpha.1`.

## 0.0.6-alpha.0

- Expanded ESP support and validation to cover all ESP architectures/chips currently targeted by `esp-hal` in this workspace flow (`esp32`, `esp32c2`, `esp32c3`, `esp32c5`, `esp32c6`, `esp32c61`, `esp32h2`, `esp32s2`, `esp32s3`) via capability-aware checks, examples, and board-generation tooling.
- Reworked ESP example generation around board profiles and templates, including chip-capability-aware example selection and improved per-chip pin/peripheral mapping.
- Improved ESP servo behavior by fixing coarse PWM quantization in `ServoEsp::set_degrees` (using raw LEDC duty counts for more accurate pulse widths).
- Improved workspace check coverage and tooling around `check-all`/xtask flows for cross-chip validation.
- Bumped workspace crate versions to `0.0.6-alpha.0`.
- Updated downstream sample projects `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to `0.0.6-alpha.0`.
- Moved `CHANGELOG.md` and `CONTRIBUTING.md` to the repository root and added `docs/release_checklist.md`.

## 0.0.4-alpha.2

- Added article link to README: "device-envoy: Making Embedded Fun with Rust, Embassy, and Composable Device Abstractions".

## 0.0.4-alpha.1

- Added new Conway's Game of Life pattern (`examples/conway.rs`).
- Added video link to README.

## 0.0.4-alpha.0

- Added first-class support for compressed audio clips (IMA ADPCM WAV) via `adpcm_clip!`.
- `AudioPlayer` now supports mixed playback of PCM clips, ADPCM clips, tones, and silence in a single `play(...)` call.
- Added `AdpcmClip`/`AdpcmClipBuf` and const conversion paths between PCM and ADPCM clip forms.
- Split clip generation into explicit `pcm_clip!` and `adpcm_clip!` flows, with generated constants including `SAMPLE_RATE_HZ`, `PCM_SAMPLE_COUNT`, and `ADPCM_DATA_LEN`.
- Improved generated docs for audio player and clip modules, including clearer generated API references.
- Added/expanded compile-only validation around resampling and sample-count invariants.

## 0.0.3-alpha.3

- Added compile-time audio resampling via `AudioClipBuf::with_resampled`.
- Added `audio_player::resampled_sample_count(...)`.
- `audio_clip!` namespaces (now split into `pcm_clip!`/`adpcm_clip!`) include `SAMPLE_RATE_HZ`, `SAMPLE_COUNT`, and `resampled_sample_count(...)`.
- Added `resampled_type!`; renamed `samples_ms!` to `samples_ms_type!`.
- `audio_player!` now generates `<Name>AudioClip` aliases (for example `AudioPlayer8AudioClip`).
- Added compile-only negative test for invalid resample destination count.
- Improved generated docs and added xtask generated-doc consistency checks.
