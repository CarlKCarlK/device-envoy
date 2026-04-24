# Conway Web Preview

This page runs the same Conway logic used by the embedded demo through WebAssembly and renders the same `Frame2d` data as PNG bytes.

The GitHub Pages root redirects to the current version at `docs/v3/`. Keep released versions in `docs/vN/` folders so old full URLs remain stable when a newer version becomes current.

## Build

Run from the repository root:

```bash
cargo build -p device-envoy-conway-wasm --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/device_envoy_conway_wasm.wasm --out-dir docs/v3/pkg --target web
```

## View

Use the VS Code Live Preview extension:

1. Open `docs/index.html` for the current version redirect, or `docs/v3/index.html` for the versioned page directly.
2. Run `Live Preview: Show Preview` from the Command Palette.
3. If the preview opens a directory URL, navigate to `/docs/` or `/docs/v3/`.

## GitHub Pages

Publish the `docs/` directory with GitHub Pages. The current redirect is:

```text
/ -> /v3/
```

When creating a new version, copy the current `docs/vN/` folder to the next version, rebuild the WASM package into that version's `pkg/` directory, and update `docs/index.html` to redirect to the new folder.

## Controls

- `Space`: play/pause
- `N`: next generation while paused
- `P`: start predecessor search
- `Esc`: cancel search
- `0`-`9`: select patterns
