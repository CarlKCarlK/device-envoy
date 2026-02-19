<!-- markdownlint-disable MD041 -->

See [`PcmClipGenerated`](crate::audio_player::pcm_clip_generated::PcmClipGenerated) for a sample of generated items.

**See the [audio_player module documentation](mod@crate::audio_player) for usage examples.**

At compile time, you can read the clip as uncompressed PCM with
[`Name::pcm_clip()`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::pcm_clip)
or compressed ADPCM with
[`Name::adpcm_clip()`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::adpcm_clip).
You can also modify the PCM data at compile time (for example with
[`Gain`](crate::audio_player::Gain) via `with_gain(...)`), and only the final
transformed clip is stored in firmware.
Additionally, you can process PCM first and still store the final clip in
compressed ADPCM form with
[`with_adpcm`](crate::audio_player::PcmClip::with_adpcm).

**Syntax:**

```text
pcm_clip! {
    [<visibility>] <Name> {
        file: <file_path_expr>,
        source_sample_rate_hz: <sample_rate_expr>,
        target_sample_rate_hz: <sample_rate_expr>, // optional, defaults to source_sample_rate_hz
    }
}
```

**Inputs:**

- `$vis` - Optional module visibility for the generated module (for example: `pub`, `pub(crate)`, `pub(self)`). Defaults to private visibility when omitted.
- `$name` - Module name for the generated module (for example: `Nasa`)

**Required fields:**

- `file` - Path to an external audio file (for example: `"nasa_22k.s16"`)
- `source_sample_rate_hz` - Source sample rate in hertz for the input file (for example: [`VOICE_22050_HZ`](crate::audio_player::VOICE_22050_HZ))

**Optional fields:**

- `target_sample_rate_hz` - Output sample rate in hertz for generated clips (default: `source_sample_rate_hz`)

**Generated items:**

- [`Name::pcm_clip()`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::pcm_clip) - `const` function that returns the uncompressed (PCM) version of this clip.
- [`Name::adpcm_clip()`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::adpcm_clip) - `const` function that returns the compressed (ADPCM) encoding for this clip.
- [`Name::SAMPLE_RATE_HZ`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::SAMPLE_RATE_HZ) - sample rate for generated clips
- [`Name::PCM_SAMPLE_COUNT`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::PCM_SAMPLE_COUNT) - number of samples for uncompressed (PCM) version of this clip
- [`Name::ADPCM_DATA_LEN`](crate::audio_player::pcm_clip_generated::PcmClipGenerated::ADPCM_DATA_LEN) - byte length for compressed (ADPCM) encoding this clip

See [`PcmClipGenerated`](crate::audio_player::pcm_clip_generated::PcmClipGenerated) and the [audio_player module documentation](mod@crate::audio_player).

# Preparing audio files for `pcm_clip!`

This macro expects audio in a simple raw format:

- mono
- 16-bit signed samples
- little-endian
- a fixed sample rate (for example, 22050 Hz)

The easiest way to produce that format is with `ffmpeg`.
