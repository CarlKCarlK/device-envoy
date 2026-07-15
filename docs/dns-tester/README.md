# DNS Tester Web Preview

This page is the browser companion to the hardware DNS tester. It uses the
`CydWasm` display/touch implementation, orientation controls, and an
independent `FlashBlockWasm` local-storage record.

## Build

Run from the repository root. Pass a new version directory when publishing a
new release so older version URLs remain available.

```bash
just build-dns-tester v1
```

Serve `docs/` over HTTP for local preview because ES-module WASM loading does
not work from a `file:` URL, for example:

```bash
just run-dns-tester v1 8000
```

Then open `http://localhost:8000/`. This serves the selected version directly.

The copied presentation resources are from Linkage Blaze skeleton-clock `v3`:
`pages/demos/skeleton-clock/v3/case.png`, `desk.jpg`, `demo-ux.css`, and
`demo-ux.js`. The stage, case, cord, BOOT, scaling, gallery card, and device
mode settings follow the Linkage Blaze landscape CYD examples.

They are checked into this version directory so future Linkage Blaze changes
cannot alter this historical Device Envoy page. The copied CSS retains the
Linkage Blaze attribution comment and project licensing applies to these
resources.
