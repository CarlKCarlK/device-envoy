# Development Guide

This guide is for editing the `device-envoy` workspace itself.

If you only want to use `device-envoy-rp` or `device-envoy-esp` in your own project, start from a template:

- [`device-envoy-blinky-rp`](https://github.com/CarlKCarlK/device-envoy-blinky-rp)
- [`device-envoy-blinky-esp`](https://github.com/CarlKCarlK/device-envoy-blinky-esp)

## Toolchains and Targets

You may need all four embedded targets, depending on what you build:

- RP Pico 1 (ARM): `thumbv6m-none-eabi`
- RP Pico 2 (ARM): `thumbv8m.main-none-eabihf`
- ESP32-C6 (RISC-V): `riscv32imac-unknown-none-elf`
- ESP32-S3 (Xtensa): `xtensa-esp32s3-none-elf`

Install Rust targets:

```bash
rustup target add thumbv6m-none-eabi
rustup target add thumbv8m.main-none-eabihf
rustup target add riscv32imac-unknown-none-elf
```

ESP32-S3 requires the ESP Rust toolchain/runtime flow used by this repo:

- Install/use the `+esp` toolchain as needed for Xtensa builds.
- Load your ESP environment before S3 commands (for this repo that is usually `source "$HOME/export-esp.sh"`).
- Use `just` recipes for S3 run/check commands in `crates/device-envoy-esp/justfile`.

## Run Examples

Use this alias naming scheme:

- `cargo <name-of-example>`: Pico 1 (default; or ESP C6 where a root alias exists)
- `cargo <name-of-example>-2`: Pico 2
- `cargo <name-of-example>-w`: Pico 1 with WiFi
- `cargo <name-of-example>-2w`: Pico 2 with WiFi
- `just <name-of-example>-s3`: ESP32-S3

Examples:

```bash
cargo blinky
cargo blinky-2
cargo clock-lcd-w
cargo clock-lcd-2w
cd crates/device-envoy-esp && just led-example1-trait-s3
```

## Run Full Workspace Checks

From the workspace root:

```bash
just check-all
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

ESP defaults:

- `GPIO2`: 16x16 panel examples and singular `led_example1_trait` external LED example
- `GPIO6`: Button input (`PressedTo::Ground` in examples)
- `GPIO7`: IR receiver data input
- Built-in NeoPixel-style (WS2812) RGB LED: `GPIO8` on ESP32-C6-DevKitC-1, `GPIO48` on ESP32-S3-DevKitC-1
- `GPIO10`: External 8-pixel NeoPixel-style (WS2812) strip
- `GPIO11`: I2S bit clock (`BCLK`)
- `GPIO12`: I2S word select (`WS` / `LRCLK`)
- `GPIO18`: 12x8 panel examples
- `GPIO21`: I2S serial data output (`DIN`)
