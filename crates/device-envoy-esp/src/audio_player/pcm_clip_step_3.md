## 3) Convert to raw PCM (`.s16`) for `pcm_clip!`

This produces a file you can embed and process at compile time:

```bash
ffmpeg -y -i nasa.mp3 -vn -ac 1 -ar 22050 -f s16le nasa_22k.s16
```

What the arguments mean:

- `-i nasa.mp3` - input file
- `-vn` - ignore any video track (safe even for audio-only inputs)
- `-ac 1` - force mono
- `-ar 22050` - set the sample rate (Hz)
- `-f s16le` - write raw 16-bit little-endian PCM (no WAV header)
- `nasa_22k.s16` - output file (ready for `pcm_clip!`)

Tip: pass the file's native rate as `source_sample_rate_hz:` and optionally resample at compile time with `target_sample_rate_hz:`.
