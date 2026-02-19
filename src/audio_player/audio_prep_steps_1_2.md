## 1) Download an example clip (NASA)

Download the MP3:

```bash
curl -L -o nasa.mp3 "https://www.nasa.gov/wp-content/uploads/2015/01/640149main_Computers20are20in20Control.mp3"
```

(Windows 10/11 includes `curl` by default.)

## 2) Install `ffmpeg` (or confirm it is installed)

General: see the official download page:

```text
https://ffmpeg.org/download.html
```

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install ffmpeg
ffmpeg -version
```

### Windows (recommended: `pixi`)

1. Install `pixi`:

```text
https://pixi.sh
```

2. Install `ffmpeg` globally:

```powershell
pixi global install ffmpeg
ffmpeg -version
```
