# Changelog

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
