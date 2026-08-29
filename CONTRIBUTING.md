# Contributing

This file explains contributor workflows that are easy to miss from individual command names.

## Validation Before a Pull Request

Run the workspace's local CI equivalent from the repository root:

```bash
cargo check-all
```

For documentation changes, also build the one authoritative review snapshot:

```bash
just docs
```

For release pull requests, follow the canonical
[release checklist](docs/release_checklist.md), which includes the additional
starter-repository and GitHub CI gates.

## PNG Snapshot Workflow (Host-Only)

The project has host-side tests that compare rendered output against checked-in PNG snapshots.

These recipes live in the RP crate's justfile. Run them from the repository
root with the explicit justfile path:

- `just --justfile crates/device-envoy-rp/justfile pngs-check-all`
- `just --justfile crates/device-envoy-rp/justfile pngs-update-led2d-graphics`
- `just --justfile crates/device-envoy-rp/justfile pngs-update-all`
- `just --justfile crates/device-envoy-rp/justfile regenerate-text-pngs`

### What Each Command Does

- `pngs-check-all`: Validates PNG snapshot tests without modifying expected files.
- `pngs-update-led2d-graphics`: Refreshes only the `led2d_graphics` expected PNG output.
- `pngs-update-all`: Refreshes all PNG expected outputs covered by the `pngs` test suite.
- `regenerate-text-pngs`: Generates text-render PNGs for manual inspection during text rendering work.

### When To Use Them

1. Run `pngs-check-all` when changing rendering behavior and you want only the PNG snapshot checks.
2. If snapshot failures are intentional due to rendering changes, use `pngs-update-led2d-graphics` for a targeted `led2d_graphics` change.
3. Use `pngs-update-all` when rendering changes intentionally affect many PNG snapshot tests.
4. Use `regenerate-text-pngs` when iterating specifically on text rendering and you want generated PNGs for manual visual inspection.
5. Do not run update commands for unrelated refactors, formatting-only changes, or other non-rendering edits.

## Example Build and UF2 Commands

These helper recipes also live in the RP crate's justfile. Use them when
validating examples on target boards or preparing UF2 artifacts for manual
hardware testing:

- `just --justfile crates/device-envoy-rp/justfile example <name>`: Build an
  example for Pico 2 (ARM).
- `just --justfile crates/device-envoy-rp/justfile example-wifi <name>`: Build
  an example for Pico 2 (ARM) with WiFi.
- `just --justfile crates/device-envoy-rp/justfile example-pico1 <name>`: Build
  an example for Pico 1 (ARM) with WiFi.
- `just --justfile crates/device-envoy-rp/justfile uf2 <name>`: Build a UF2
  image for Pico 2 (ARM).
- `just --justfile crates/device-envoy-rp/justfile uf2-wifi <name>`: Build a UF2
  image for Pico 2 (ARM) with WiFi.

Examples:

- `just --justfile crates/device-envoy-rp/justfile example led_strip`
- `just --justfile crates/device-envoy-rp/justfile example-wifi wifi_auto`
- `just --justfile crates/device-envoy-rp/justfile uf2 blinky`

For full options and command behavior, run:

```bash
cd crates/device-envoy-rp
cargo xtask --help
```

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.
