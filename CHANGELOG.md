# Changelog

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
