Macro to define an ADPCM clip namespace from a WAV file.

**Syntax:**

```text
adpcm_clip! {
    [<visibility>] <Name> {
        file: <path_expr>,
        target_sample_rate_hz: <sample_rate_expr>, // optional, defaults to WAV sample_rate_hz
    }
}
```

**Inputs:**

- `$vis` - Optional generated module visibility.
- `$name` - Module name for the generated namespace.

**Required fields:**

- `file` - Path to an ADPCM WAV file.

**Optional fields:**

- `target_sample_rate_hz` - Output sample rate in hertz for generated clips (default: the WAV file sample rate).

**Generated items:**

- `<Name>::SOURCE_SAMPLE_RATE_HZ`
- `<Name>::TARGET_SAMPLE_RATE_HZ`
- `<Name>::SAMPLE_RATE_HZ`
- `<Name>::PCM_SAMPLE_COUNT`
- `<Name>::ADPCM_DATA_LEN`
- `<Name>::adpcm_clip()`
- `<Name>::pcm_clip()`
- `<Name>::pcm_clip_from(...)`
- `<Name>::adpcm_clip_from(...)`
- `<Name>::with_gain(...)`

See the [audio_player module documentation](mod@crate::audio_player) for usage examples.

# Preparing audio files for `adpcm_clip!`

This macro expects mono IMA ADPCM WAV input.
