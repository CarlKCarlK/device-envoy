# device-envoy-esp

<!-- todo0 create and publish template repo: device-envoy-blinky-esp -->
<!-- todo0 create and publish template repo: device-envoy-blinky-rp -->

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)
[![crates.io](https://img.shields.io/crates/v/device-envoy-esp?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy-esp)
[![docs.rs](https://img.shields.io/docsrs/device-envoy-esp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-esp)

**Build ESP32 applications with composable device abstractions.**

`device-envoy-esp` is an embedded Rust library built on Embassy and esp-hal.
It organizes hardware around device abstractions so application code can use
small, focused APIs instead of managing low-level coordination directly.

Currently targeting ESP32-C6 and ESP32-S3 in [`device-envoy-esp`](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/), and Raspberry Pi Pico 1 and Pico 2 (ARM cores) in [`device-envoy-rp`](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/).

## Start From a Template

Want a minimal starting project?

- [`device-envoy-blinky-esp` on GitHub](https://github.com/CarlKCarlK/device-envoy-blinky-esp)

## Status

⚠️ **Alpha / Experimental**

The API is actively evolving and may change without compatibility guarantees.

## Features

- **[LED Strips](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/led_strip/) & [Panels](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/led2d/)** - NeoPixel-style (WS2812) LED arrays with 2D text rendering, animation, embedded-graphics support. Provides efficient options for power limiting and color correction.
- **[WiFi](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/wifi_auto/)** - Connect to the Internet with automatic credentials management. On boot, opens a web form if WiFi credentials aren't saved, then connects seamlessly to a stored network.
- **[Audio Player](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/audio_player/)** - Play audio clips over I²S hardware with runtime sequencing, volume control, and compression.
- **[Button Input](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/button/)** - Button handling with debouncing
- **[Servo Control](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/servo/)** - Servo positioning and animation
- **[Flash Storage](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/flash_block/)** - Type-safe, on-board persistent storage
- **[LCD Display](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/lcd_text/)** - Text display (HD44780)
- **[IR Remote](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/ir/)** - Remote control decoder (NEC protocol)
- **[RFID Reader](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/rfid/)** - Card detection and reading (MFRC522)
- **[Clock Sync](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/clock_sync/)** - Network time synchronization utilities
- **[LED4 Display](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/led4/)** - 4-digit, 7-segment LED display control with optional animation and blinking
- **[Single LED](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/led/)** - Single LED control with animation support

## Forum

- **[Using Embassy to build applications](https://github.com/CarlKCarlK/device-envoy/discussions)**  
  A place to talk about writing embedded applications with Embassy: sharing code, asking practical questions, and learning what works in practice.  
  Not limited to Pico or ESP boards, or to `device-envoy`.

## Videos and Articles

- [device-envoy: Making Embedded Fun with Rust, Embassy, and Composable Device Abstractions](https://medium.com/@carlmkadie/device-envoy-making-embedded-fun-31534917414b) -- versions: [article](https://medium.com/@carlmkadie/device-envoy-making-embedded-fun-31534917414b) or [video](https://www.youtube.com/watch?v=iUu6hvJLVOU)
- [How Rust & Embassy Shine on Embedded Devices](https://medium.com/@carlmkadie/how-rust-embassy-shine-on-embedded-devices-part-1-9f4911c92007) by Carl M. Kadie and Brad Gibson.
- [More Rust articles](https://medium.com/@carlmkadie)

## Examples & Demos

The project includes **examples** (single-device tests) in `examples/` showing integration patterns:

### Example: animated LED strip

This example cycles a 96-LED strip through red, green, and blue frames.

![Animated 96-LED strip example (APNG)](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led_strip_animated.png)

It shows how device-envoy generates a struct (device abstraction) for an LED
strip and then animates a sequence of frames.

```rust,no_run
# #![no_std]
# #![no_main]
# use esp_backtrace as _;
# use core::convert::Infallible;
use device_envoy_esp::{Result, init_and_start, led_strip, led_strip::{LedStrip as _, Frame1d, colors}};
use embassy_time::Duration;

led_strip! {
    LedStripAnimated {
        pin: GPIO18,
        len: 96,
    }
}

async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking);
    let led_strip_animated = LedStripAnimated::new(p.GPIO18, rmt80.channel0, spawner)?;

    // Create a sequence of frames and durations and then animate them (looping, until replaced).
    let frame_duration = Duration::from_millis(300);
    led_strip_animated.animate([
        (Frame1d::filled(colors::RED), frame_duration),
        (Frame1d::filled(colors::GREEN), frame_duration),
        (Frame1d::filled(colors::BLUE), frame_duration),
    ]);

    core::future::pending().await // run forever
}
```

> For complete, runnable examples (including wiring and setup), see the `examples/` directory.

- **Basic LED Examples**: Simple on/off control with blinky pattern
- **LED Strip Examples**: Simple animations, color control, text rendering
- **LED Panel Examples**: 12×8, 16×16, and multi-panel configurations with graphics

![Animated LED panel Go Go example](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led2d2.png)

- **Button Examples**: Debouncing and state handling
- **Servo Examples**: Position sweeps and animation playback
- **WiFi Examples**: WiFi setup, time sync, DNS
- **Flash Examples**: Configuration persistence and data reset

See the `examples/` directory for complete runnable code.

## Building & Running

### Prerequisites

```bash
rustup target add riscv32imac-unknown-none-elf
```

### Quick Start

```bash
# New project template
# https://github.com/CarlKCarlK/device-envoy-blinky-esp

# Check this crate
cargo check

# Run an example (adjust board features as needed)
cargo run --example led_example1_trait --target riscv32imac-unknown-none-elf
```

**Tools:**

- `just` - Optional command runner (install with `cargo install just` or your package manager). See `justfile` for commands.
- `xtask` - Project's custom automation tool (built-in, use via `cargo xtask --help`)

See `.cargo/config.toml` for cargo aliases.

## Hardware Notes

### Standard Pinouts

This section is the single source of truth for default pin assignments used by
examples in this repo.

- `GPIO6` - Button input (wired to GND in examples using `PressedTo::Ground`)
- `GPIO7` - IR receiver data input
- `GPIO11` - I2S bit clock (`BCLK`)
- `GPIO12` - I2S word select (`WS` / `LRCLK`)
- `GPIO21` - I2S serial data output (`DIN`)
- Built-in NeoPixel-style (WS2812) RGB LED (board-specific):
- `GPIO8` on ESP32-C6-DevKitC-1
- `GPIO48` on ESP32-S3-DevKitC-1
- `GPIO10` - External 8-pixel NeoPixel-style (WS2812) strip
- `GPIO18` - 12x8 panel examples
- `GPIO2` - 16x16 panel examples and the singular `led_example1_trait` external LED example

`GPIO11` and `GPIO12` replaced the previous `GPIO22`/`GPIO23` defaults because
`GPIO22`-`GPIO25` are not exposed as peripherals in esp-hal on ESP32-S3.

### Peripheral Resources

- `I2S0 + DMA_CH0` - Audio examples
- `SPI2` - SPI-driven LED panel/strip examples
- ESP32-C6 RMT IR receive channel: `channel2` (channels 0-3 all support RX)
- ESP32-S3 RMT IR receive channel: `channel4` (channels 0-3 are TX-only; RX requires channel 4+)

### Portability Guardrails

- Avoid C6 USB pins: `GPIO12`, `GPIO13`
- Avoid S3 USB pins: `GPIO19`, `GPIO20`
- Avoid C6 strapping pins for defaults: `GPIO4`, `GPIO5`, `GPIO8`, `GPIO9`, `GPIO15`
- Avoid S3 strapping pins for defaults: `GPIO0`, `GPIO3`, `GPIO45`, `GPIO46`
- Avoid S3 unavailable pins: `GPIO22`-`GPIO25` are not exposed in esp-hal for ESP32-S3
- Avoid flash/SPI0/1-connected pins as defaults on board variants where they are not GPIO-safe

Note: `GPIO8` appears both as a built-in LED pin and in the C6 strapping-pin
avoid list. That is intentional: keep it for the board's built-in LED only, and
avoid using it as a default for new external signals.

## Testing

Host-side checks and tests:

```bash
cargo check
cargo test
```

`just` is the optional command runner (install with `cargo install just` or your package manager). See **Tools** above.

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.

## License

Licensed under either:

- MIT license (see LICENSE-MIT file)
- Apache License, Version 2.0

at your option.
