# Contributing

This file explains contributor workflows that are easy to miss from individual command names.

## PNG Snapshot Workflow (Host-Only)

The project has host-side tests that compare rendered output against checked-in PNG snapshots.

Use these commands:

- `just verify-all`
- `just pngs-check-all`
- `just pngs-update-led2d-graphics`
- `just pngs-update-all`
- `just regenerate-text-pngs`

### What Each Command Does

- `just verify-all`: Runs full validation in one command: docs update/build, embedded checks, and PNG snapshot comparison checks.
- `just pngs-check-all`: Validates PNG snapshot tests without modifying expected files.
- `just pngs-update-led2d-graphics`: Refreshes only the `led2d_graphics` expected PNG output.
- `just pngs-update-all`: Refreshes all PNG expected outputs covered by the `pngs` test suite.
- `just regenerate-text-pngs`: Generates text-render PNGs for manual inspection during text rendering work.

### When To Use Them

1. Run `just verify-all` before opening a PR when you want a full local validation pass.
2. Run `just pngs-check-all` when changing rendering behavior and you want only the PNG snapshot checks.
3. If snapshot failures are intentional due to rendering changes, update expected files with `just pngs-update-led2d-graphics` for a targeted `led2d_graphics` change.
4. Use `just pngs-update-all` when rendering changes intentionally affect many PNG snapshot tests.
5. Use `just regenerate-text-pngs` when iterating specifically on text rendering and you want generated PNGs for manual visual inspection.
6. Do not run update commands for unrelated refactors, formatting-only changes, or other non-rendering edits.

## Example Build and UF2 Commands

Use these helper commands when validating examples on target boards or preparing UF2 artifacts for manual hardware testing:

- `just example <name>`: Build an example for Pico 2 (ARM).
- `just example-wifi <name>`: Build an example for Pico 2 (ARM) with WiFi.
- `just example-pico1 <name>`: Build an example for Pico 1 (ARM) with WiFi.
- `just uf2 <name>`: Build a UF2 image for Pico 2 (ARM).
- `just uf2-wifi <name>`: Build a UF2 image for Pico 2 (ARM) with WiFi.

Examples:

- `just example led_strip`
- `just example-wifi wifi_auto`
- `just uf2 blinky`

For full options and command behavior, see `cargo xtask --help`.

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.
