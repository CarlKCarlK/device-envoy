# Audio With-Gain Golden Files

These files are host-test fixtures for `src/audio_player/host_tests.rs`.

- `silence_32.s16`: expected output for `AudioClipBuf::silence()` with 32 samples
- `tone_440hz_32.s16`: expected output for `AudioClipBuf::tone(440)` with 32 samples
- `tone_440hz_32_gain_50.s16`: expected output for `tone(...).with_gain(Gain::percent(50))`
- `tone_440hz_32_gain_200.s16`: expected output for `tone(...).with_gain(Gain::percent(200))`

To regenerate expected files from the current implementation:

```bash
DEVICE_KIT_UPDATE_AUDIO=1 cargo test --features host --lib audio_player::host_tests
```
