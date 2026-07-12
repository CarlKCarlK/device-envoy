# Conway Web Preview

This page runs the same Conway logic used by the embedded demo through WebAssembly and renders the same `Frame2d` data as PNG bytes.

The GitHub Pages root redirects to the current version at `docs/conway/v2/`.
Keep released versions in `docs/conway/vN/` folders so old URLs remain stable
when a newer version becomes current.

## Build

Run from the repository root:

```bash
just build-conway v2
```

## View

Use the built-in local server:

```bash
just run-conway v2 8000
```

Then open `http://localhost:8000/`. This serves the selected version directly.

## GitHub Pages

Publish the `docs/` directory with GitHub Pages. The current redirect is:

```text
/conway/ -> /conway/v2/
```

When creating a new version, copy the previous `docs/conway/vN/` directory,
rebuild the WASM package into its `pkg/` directory, and update
`docs/conway/index.html` to redirect to the new version.

## Controls

- `Space`: play/pause
- `N`: next generation while paused
- `P`: start predecessor search
- `Esc`: cancel search
- `0`-`9`: select patterns
