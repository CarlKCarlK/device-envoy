<!-- markdownlint-disable MD041 -->

See [`AdpcmClipGenerated`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated) for a sample of generated items.

At compile time, you can read the clip as compressed ADPCM with
[`Name::adpcm_clip()`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::adpcm_clip)
or uncompressed PCM with
[`Name::pcm_clip()`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::pcm_clip).
You can also modify the PCM data at compile time (for example with
[`Gain`](crate::audio_player::Gain) via `with_gain(...)`), and only the final
transformed clip is stored in firmware.
Additionally, you can decode to PCM first with
[`with_pcm`](crate::audio_player::AdpcmClip::with_pcm), process it, and still
store the final clip in compressed ADPCM form with
[`with_adpcm`](crate::audio_player::PcmClip::with_adpcm).

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

- [`Name::pcm_clip()`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::pcm_clip) - const function that returns the generated audio clip
- [`Name::adpcm_clip()`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::adpcm_clip) - const function that returns the generated clip encoded as ADPCM
- [`Name::SAMPLE_RATE_HZ`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::SAMPLE_RATE_HZ) - sample rate for generated clips
- [`Name::ADPCM_DATA_LEN`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::ADPCM_DATA_LEN) - byte length for compressed (ADPCM) encoding this clip
- [`Name::PCM_SAMPLE_COUNT`](crate::audio_player::adpcm_clip_generated::AdpcmClipGenerated::PCM_SAMPLE_COUNT) - number of samples for uncompressed (PCM) version of this clip

See the [audio_player module documentation](mod@crate::audio_player) for usage examples.

# Preparing audio files for `adpcm_clip!`

This macro expects mono IMA ADPCM WAV input.
