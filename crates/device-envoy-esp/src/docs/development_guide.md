# Development Guide

This guide is for editing the `device-envoy` workspace itself.

If you only want to use `device-envoy-rp` or `device-envoy-esp` in your own project, start from a template:

- [`device-envoy-rp-blinky`](https://github.com/CarlKCarlK/device-envoy-rp-blinky)
- [`device-envoy-esp-blinky`](https://github.com/CarlKCarlK/device-envoy-esp-blinky)

## Toolchains and Targets

You may need all embedded targets below, depending on what you build:

- RP Pico 1 (ARM): `thumbv6m-none-eabi`
- RP Pico 2 (ARM): `thumbv8m.main-none-eabihf`
- ESP32-C5 / ESP32-C6 / ESP32-C61 / ESP32-H2 (RISC-V): `riscv32imac-unknown-none-elf`
- ESP32-C2 / ESP32-C3 (RISC-V): `riscv32imc-unknown-none-elf`
- ESP32 / ESP32-S2 / ESP32-S3 (Xtensa):
  `xtensa-esp32-none-elf`, `xtensa-esp32s2-none-elf`, `xtensa-esp32s3-none-elf`

Install Rust targets:

```bash
rustup target add thumbv6m-none-eabi
rustup target add thumbv8m.main-none-eabihf
rustup target add riscv32imc-unknown-none-elf
rustup target add riscv32imac-unknown-none-elf
```

Xtensa ESP chips (ESP32 / ESP32-S2 / ESP32-S3) require the ESP Rust
toolchain/runtime flow used by this repo:

- Install/use the `+esp` toolchain as needed for Xtensa builds.
- Load your ESP environment before S3 commands (for this repo that is usually `source "$HOME/export-esp.sh"`).
- Use `just run/check/build` commands in `crates/device-envoy-examples-esp/justfile`.

## Run Examples

Use `just` from the crate you are in (`crates/device-envoy-rp` or `crates/device-envoy-esp`):

- `just run <name> [target]`
- `just check <name> [target]`
- `just build <name> [target]`

Defaults:

- ESP crate default target: `c6`
- RP crate default target: `1`

Common target suffixes:

- ESP: `c2`, `c3`, `c5`, `c6`, `c61`, `h2`, `esp32`, `s2`, `s3`
- RP: `1`, `2`, `w`, `2w`, `2r`

Examples:

```bash
cd crates/device-envoy-esp
cd examples/c6/devkitc1_n8
just run blinky
cd ../../..
cd examples/s3/generic
just run blinky
cd ../../..
just build f1 s3

cd ../device-envoy-rp
just run blinky
just run blinky 2
just check f1 2w
```

For board-generated ESP blinkies, the preferred flow is to run from the board directory:

```bash
cd crates/device-envoy-examples-esp/examples/<chip-dir>/<board-dir>
just run blinky
```

### ESP board example generation

Board-specific ESP examples are generated files under `examples/<chip>/<board>/`.

Source of truth:

- Board profiles and pin mappings: `xtask/src/boards.rs` (`BOARD_PROFILES`)
- Board example generation/rendering logic: `xtask/src/board_examples_generated.rs`
- Templates: `examples/templates/blinky_plain.rs.j2`, `examples/templates/blinky_rmt.rs.j2`, `examples/templates/blinky_spi.rs.j2`, `examples/templates/led16x16_plus_1.rs.j2`, `examples/templates/led16x16_plus_1_spi.rs.j2`

Regenerate after changing templates or board mappings:

```bash
cd crates/device-envoy-esp
just generate-board-examples
```

## Run Full Workspace Checks

From the workspace root:

```bash
cargo check-all
```

Equivalent command:

```bash
cargo run --manifest-path xtask/Cargo.toml -- check-all
```

## Generate Docs

From the workspace root:

```bash
just update-docs-rp
just update-docs-esp
```

These are the authoritative documentation builds. They regenerate and stage
documentation images and validate the rendered output. The explicitly named
`update-docs-rp-no-images`, `update-docs-esp-unvalidated`, and
`update-docs-incomplete` recipes are incomplete local previews; each prints a
warning and must not be used for documentation review or publishing.

Optional: build and open docs in a browser:

```bash
just show-docs-rp
just show-docs-esp
```

Note: `show-docs-rp` and `show-docs-esp` are currently WSL/Windows-oriented scripts.

## Standard Pin Assignments

These are the default pins used by examples in this repository.

RP defaults:

- `PIN_0`: LED strip (8-pixel simple example)
- `PIN_1`: Single LED (blinky patterns)
- `PIN_3`: LED panel (12x4, 48 pixels)
- `PIN_4`: Extended LED panel (12x8, 96 pixels)
- `PIN_5`: Long LED strip (160 pixels, marquee effects)
- `PIN_6`: Large LED panel (16x16, 256 pixels)
- `PIN_8`: I2S audio data (`DIN`)
- `PIN_9`: I2S bit clock (`BCLK`)
- `PIN_10`: I2S word select (`LRC` / `LRCLK`)
- `PIN_11`, `PIN_12`: Servo signals
- `PIN_13`: Button (active-low)

ESP defaults are chip- and board-dependent.

For current source-of-truth pin/capability mappings, see:

- `crates/device-envoy-examples-esp/xtask/src/boards.rs` (`BOARD_PROFILES`)
- `crates/device-envoy-examples-esp/xtask/src/main.rs` (capability-based example/test gating)
- Generated board examples under `crates/device-envoy-examples-esp/examples/<chip>/<board>/`
