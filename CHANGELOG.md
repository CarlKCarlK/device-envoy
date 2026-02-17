# Changelog

## 0.0.3-alpha.3

- Added compile-time audio resampling via `AudioClipBuf::with_resampled`.
- Added `audio_player::resampled_sample_count(...)`.
- `audio_clip!` namespaces (now split into `pcm_clip!`/`adpcm_clip!`) include `SAMPLE_RATE_HZ`, `SAMPLE_COUNT`, and `resampled_sample_count(...)`.
- Added `resampled_type!`; renamed `samples_ms!` to `samples_ms_type!`.
- `audio_player!` now generates `<Name>AudioClip` aliases (for example `AudioPlayer8AudioClip`).
- Added compile-only negative test for invalid resample destination count.
- Improved generated docs and added xtask generated-doc consistency checks.
