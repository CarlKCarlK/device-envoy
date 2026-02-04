# device-envoy

**Build Pico applications with LED panels, easy WiFi, and composable device abstractions.**

`device-envoy` explores application-level device abstractions in embedded Rust using the Embassy async framework. It focuses on building reusable, typed async APIs that hide timing, interrupts, channels, and shared state inside the device.

Currently targeting Raspberry Pi Pico 1 and Pico 2 (ARM cores). RISC-V core support exists but is not actively tested.

## Status

⚠️ **Alpha / Experimental** (version 0.0.2-alpha)

The API is actively evolving. Not recommended for production use, but excellent for experimentation and embedded Rust learning.

**Background:** See [How Rust & Embassy Shine on Embedded Devices](https://medium.com/@carlmkadie/how-rust-embassy-shine-on-embedded-devices-part-1-9f4911c92007) by Carl M. Kadie and Brad Gibson.

## Features

- **LED Panels & Strips** - NeoPixel-style (WS2812) LED arrays with 2D text rendering, animation, and embedded-graphics support
- **WiFi (Pico W)** - Connect to the Internet with automatic credentials management. On boot, opens a web form if WiFi credentials aren't saved, then connects seamlessly to stored networks.
- **Button Input** - Button handling with debouncing
- **Servo Control** - Servo positioning and animation
- **Flash Storage** - Type-safe, on-board persist storage
- **LCD Display** - Text display (HD44780)
- **IR Remote** - Remote control decoder (NEC protocol)
- **RFID Reader** - Card detection and reading (MFRC522)

## Examples & Demos

The project includes **examples** (single-device tests) in `examples/` and **demo applications** in `demos/` showing real-world integration patterns:

- **LED Strip Examples**: Simple animations, color control, text rendering
- **LED Panel Examples**: 12×4, 12×8, and multi-panel configurations with graphics
- **Button Examples**: Debouncing and state handling
- **Servo Examples**: Position sweeps and animation playback
- **WiFi Examples**: WiFi setup, time sync, DNS
- **Flash Examples**: Configuration persistence and data reset

See the `examples/` and `demos/` directories for complete runnable code.

## Building & Running

### Prerequisites

```bash
# Add Rust targets for Pico boards (ARM cores are fully tested)
rustup target add thumbv6m-none-eabi           # Pico 1 (ARM)
rustup target add thumbv8m.main-none-eabihf    # Pico 2 (ARM)

# Optional: Pico 2 RISC-V core (not actively tested, experimental)
rustup target add riscv32imac-unknown-none-elf
```

### Build the library

```bash
# Build just the library for Pico 1 (ARM core, no WiFi)
cargo build --lib --target thumbv6m-none-eabi

# Pico 2 with WiFi (requires --no-default-features to override pico1 default)
cargo build --lib --target thumbv8m.main-none-eabihf --features pico2,wifi,arm --no-default-features
```

**Feature flags explained:**

- `pico1` / `pico2` — Which board to target
- `arm` — Use ARM core (default on both boards)
- `wifi` — Enable WiFi module (Pico W only)

### Run examples

Examples are easiest to run using cargo aliases defined in `.cargo/config.toml`:

```bash
# Pico 1 examples (simplest)
cargo blinky

# Pico 2 examples (with WiFi)
cargo clock-lcd-w
cargo clock-led12x4-w

# Just check without running
cargo blinky-check
```

Or use `cargo xtask` for more control:

```bash
cargo xtask example blinky --board pico1 --arch arm
cargo xtask example clock_lcd --board pico2 --arch arm --wifi
```

For a complete list of cargo aliases, see `.cargo/config.toml`. For `just` commands, see `justfile`.

### Check all (builds, tests, docs)

```bash
cargo check-all  # Runs all checks in parallel
```

## Hardware Notes

### Standard Pinouts

Examples use conventional pin assignments for consistency:

- **PIN_0**: LED strip (8-pixel simple example)
- **PIN_3**: LED panel (12×4, 48 pixels)
- **PIN_4**: Extended LED panel (12×8, 96 pixels)
- **PIN_13**: Button (active-low)
- **PIN_11, PIN_12**: Servo signals

### WiFi (Pico W)

- **PIN_23**: CYW43 power enable
- **PIN_24–29**: CYW43 SPI + control pins (via PIO)

## Testing

Host-side tests run on your development machine without hardware:

```bash
# Run host tests (unit + integration)
cargo test --no-default-features --features defmt,host

# Or run via xtask
cargo xtask check-docs  # Includes doc tests
```

Tests include:

- LED text rendering comparisons against reference images
- 2D LED matrix mapping algebra
- LED color space conversions

## License

Licensed under either:

- MIT license (see LICENSE-MIT file)
- Apache License, Version 2.0

at your option.
