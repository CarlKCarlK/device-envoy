## 3) Convert to ADPCM WAV (`.wav`) for `adpcm_clip!`

This produces a mono IMA ADPCM WAV that `adpcm_clip!` can compile in:

```bash
ffmpeg -y -i nasa.mp3 -vn -ac 1 -ar 22050 -c:a adpcm_ima_wav -block_size 256 nasa_22k_adpcm.wav
```

What the extra arguments mean:

- `-c:a adpcm_ima_wav` - encode IMA ADPCM in WAV
- `-block_size 256` - use 256-byte ADPCM blocks (matches `pcm_clip!` ADPCM output blocks)
- `nasa_22k_adpcm.wav` - output file (ready for `adpcm_clip!`)

Tip: omit `target_sample_rate_hz` in `adpcm_clip!` to keep the WAV sample rate, or set it to resample at compile time.
