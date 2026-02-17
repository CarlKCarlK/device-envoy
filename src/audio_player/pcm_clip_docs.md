Macro to "compile in" an audio clip from an external file (includes syntax details). See [`PcmClipGenerated`](crate::audio_player::pcm_clip_generated::PcmClipGenerated) for a sample of generated items.

**See the [audio_player module documentation](mod@crate::audio_player) for usage examples.**

The generated clip can be modified at compile time (for example with [`Gain`](crate::audio_player::Gain) via `with_gain(...)`) and only increases binary size when you store it in a `static`.

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

- `$vis` - Optional module visibility for the generated namespace (for example: `pub`, `pub(crate)`, `pub(self)`). Defaults to private visibility when omitted.
- `$name` - Module name for the generated namespace (for example: `Nasa`)

**Required fields:**

- `file` - Path to an external audio file (for example: `"nasa_22k.s16"`)
- `source_sample_rate_hz` - Source sample rate in hertz for the input file (for example: [`VOICE_22050_HZ`](crate::audio_player::VOICE_22050_HZ))

**Optional fields:**

- `target_sample_rate_hz` - Output sample rate in hertz for generated clips (default: `source_sample_rate_hz`)

**Generated items:**

- `pcm_clip()` - `const` function that returns the generated audio clip
- `adpcm_clip()` - `const` function that returns the generated clip encoded as ADPCM (256-byte blocks)
- `SAMPLE_RATE_HZ` - sample rate for generated clips
- `PCM_SAMPLE_COUNT` - number of i16 PCM samples in the generated clip
- `ADPCM_DATA_LEN` - ADPCM byte length for encoding this clip

**Mental model (lifecycle):**

Each `pcm_clip!` invocation generates:

- a module namespace
- a `const fn pcm_clip()` constructor

Audio bytes are embedded in program flash via `include_bytes!`.
The clip value can be constructed at compile time when used in `const` or `static` definitions.
When you take `&Name::pcm_clip()` in a `static` context, the compiler promotes that clip value into flash storage.

# Example

See [`PcmClipGenerated`](crate::audio_player::pcm_clip_generated::PcmClipGenerated) and the [audio_player module documentation](mod@crate::audio_player).

# Preparing audio files for `pcm_clip!`

This macro expects audio in a simple raw format:

- mono
- 16-bit signed samples
- little-endian
- a fixed sample rate (for example, 22050 Hz)

The easiest way to produce that format is with `ffmpeg`.
