# Audio Sources

## Jabberwocky clip

Source:

- https://dn710909.ca.archive.org/0/items/jabberwocky_librivox/jabberwocky_carroll_td_64kb.mp3

Local input used for conversion:

- `C:\Users\carlk\Downloads\jabberwocky_carroll_clip.mp3`

Converted outputs:

- `jabberwocky_22k.s16` (mono, 16-bit PCM, 22050 Hz)
- `jabberwocky_22k_adpcm.wav` (mono IMA ADPCM WAV, 22050 Hz, block size 256)

Commands used:

```bash
ffmpeg -y -i /mnt/c/Users/carlk/Downloads/jabberwocky_carroll_clip.mp3 -vn -ac 1 -ar 22050 -f s16le examples/data/audio/jabberwocky_22k.s16
ffmpeg -y -i /mnt/c/Users/carlk/Downloads/jabberwocky_carroll_clip.mp3 -vn -ac 1 -ar 22050 -c:a adpcm_ima_wav -block_size 256 examples/data/audio/jabberwocky_22k_adpcm.wav
```
