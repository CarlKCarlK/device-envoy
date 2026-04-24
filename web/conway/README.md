# Conway Web Preview

This page runs the same Conway logic used by the embedded demo through WebAssembly and renders the same `Frame2d` data as PNG bytes.

## Build

Run from the repository root:

```bash
cargo build -p device-envoy-conway-wasm --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/device_envoy_conway_wasm.wasm --out-dir target/wasm32-unknown-unknown/release --target web
mkdir -p web/conway/pkg
cp target/wasm32-unknown-unknown/release/device_envoy_conway_wasm.js \
  target/wasm32-unknown-unknown/release/device_envoy_conway_wasm_bg.wasm \
  target/wasm32-unknown-unknown/release/device_envoy_conway_wasm.d.ts \
  target/wasm32-unknown-unknown/release/device_envoy_conway_wasm_bg.wasm.d.ts \
  web/conway/pkg/
```

## View

Use the VS Code Live Preview extension:

1. Open `web/conway/index.html`.
2. Run `Live Preview: Show Preview` from the Command Palette.
3. If the preview opens a directory URL, navigate to `/web/conway/`.

## Controls

- `Space`: play/pause
- `N`: next generation while paused
- `P`: start predecessor search
- `Esc`: cancel search
- `0`-`9`: select patterns
