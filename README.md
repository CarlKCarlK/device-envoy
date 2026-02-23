# device-envoy

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)
[![crates.io](https://img.shields.io/crates/v/device-envoy?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy)
[![docs.rs](https://img.shields.io/docsrs/device-envoy?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy)

**Build Pico applications with LED panels, easy WiFi, and composable device abstractions.**

`device-envoy` is a library for building embedded applications in Rust, built on the Embassy framework. It organizes hardware around *device abstractions*.

A device abstraction is a software encapsulation of hardware that manages timing, tasks, control flow, interrupts, channels, and state within the abstraction.

Rather than replacing HALs or drivers, `device-envoy` builds on them. It defines device abstractions that expose a small set of simple operations to the rest of the program.

Currently targeting Raspberry Pi Pico 1 and Pico 2 (ARM cores). RISC-V core support exists but is not actively tested.

## Start From a Template

Want a minimal starting project? Use [`device-envoy-blinky` on GitHub](https://github.com/CarlKCarlK/device-envoy-blinky) as a template.

## Status

⚠️ **Alpha / Experimental**

The API is actively evolving. Not recommended for production use, but excellent for experimentation, learning, and exploratory projects.

## Features

- **[LED Strips](https://docs.rs/device-envoy/latest/device_envoy/led_strip/) & [Panels](https://docs.rs/device-envoy/latest/device_envoy/led2d/)**  - NeoPixel-style (WS2812) LED arrays with 2D text rendering, animation, embedded-graphics support. Provides efficient options for power limiting and color correction.
- **[WiFi (Pico W)](https://docs.rs/device-envoy/latest/device_envoy/wifi_auto/)** - Connect to the Internet with automatic credentials management. On boot, opens a web form if WiFi credentials aren't saved, then connects seamlessly to a stored network. Requires Pico W; WiFi is not supported on non-W boards.
- **[Audio Player](https://docs.rs/device-envoy/latest/device_envoy/audio_player/)** - Play audio clips over I²S hardware with runtime sequencing, volume control, and compression.
- **[Button Input](https://docs.rs/device-envoy/latest/device_envoy/button/)** - Button handling with debouncing
- **[Servo Control](https://docs.rs/device-envoy/latest/device_envoy/servo/)** - Servo positioning and animation
- **[Flash Storage](https://docs.rs/device-envoy/latest/device_envoy/flash_array/)** - Type-safe, on-board persist storage
- **[LCD Display](https://docs.rs/device-envoy/latest/device_envoy/char_lcd/)** - Text display (HD44780)
- **[IR Remote](https://docs.rs/device-envoy/latest/device_envoy/ir/)** - Remote control decoder (NEC protocol)
- **[RFID Reader](https://docs.rs/device-envoy/latest/device_envoy/rfid/)** - Card detection and reading (MFRC522)
- **[Clock Sync](https://docs.rs/device-envoy/latest/device_envoy/clock_sync/)** - Network time synchronization utilities
- **[LED4 Display](https://docs.rs/device-envoy/latest/device_envoy/led4/)** - 4-digit, 7-segment LED display control with optional animation and blinking
- **[Single LED](https://docs.rs/device-envoy/latest/device_envoy/led/)** - Single LED control with animation support

## Forum

- **[Using Embassy to build applications](https://github.com/CarlKCarlK/device-envoy/discussions)**  
  A place to talk about writing embedded applications with Embassy: sharing code, asking practical questions, and learning what works in practice.  
  Not limited to Pico boards or to `device-envoy`.

## Videos and Articles

- &#91;video&#93; [device-envoy: Making Embedded Fun with Rust, Embassy, and Composable Device Abstractions](https://www.youtube.com/watch?v=iUu6hvJLVOU) (Seattle Rust User Group)
- [How Rust & Embassy Shine on Embedded Devices](https://medium.com/@carlmkadie/how-rust-embassy-shine-on-embedded-devices-part-1-9f4911c92007) by Carl M. Kadie and Brad Gibson.
- [More Rust articles](https://medium.com/@carlmkadie)

## Examples & Demos

The project includes **examples** (single-device tests) in `examples/` and **demo applications** in `demos/` showing integration patterns:

### Example: animated LED strip

This example cycles a 96-LED strip through red, green, and blue frames.
![Animated 96-LED strip example (APNG)](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led_strip_animated.png)

It shows how device-envoy generates a struct (device abstraction) for an LED strip and then animates a sequence of frames.

```rust,no_run
# #![no_std]
# #![no_main]
# use panic_probe as _;
# use core::convert::Infallible;
use device_envoy::{Result, led_strip::{Frame1d, colors}};
use device_envoy::led_strip;

led_strip! {
    LedStripAnimated {
        pin: PIN_4,
        len: 96,
    }
}

async fn example(spawner: embassy_executor::Spawner) -> Result<Infallible> {
    let p = embassy_rp::init(Default::default());
    let led_strip_animated = LedStripAnimated::new(p.PIN_4, p.PIO0, p.DMA_CH0, spawner)?;

    // Create a sequence of frames and durations and then animate them (looping, until replaced).
    let frame_duration = embassy_time::Duration::from_millis(300);
    led_strip_animated.animate([
        (Frame1d::filled(colors::RED), frame_duration),
        (Frame1d::filled(colors::GREEN), frame_duration),
        (Frame1d::filled(colors::BLUE), frame_duration),
    ])?;

    core::future::pending().await // run forever
}
```

> For complete, runnable examples (including wiring and setup), see the `examples/` and `demos/` directories.

- **Basic LED Examples**: Simple on/off control with blinky pattern
- **LED Strip Examples**: Simple animations, color control, text rendering

- **LED Panel Examples**: 12×4, 12×8, and multi-panel configurations with graphics

![Animated LED panel Go Go example](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/docs/assets/led2d2.png)

- **Button Examples**: Debouncing and state handling
- **Servo Examples**: Position sweeps and animation playback
- **WiFi Examples**: WiFi setup, time sync, DNS
- **Flash Examples**: Configuration persistence and data reset

See the `examples/` and `demos/` directories for complete runnable code.

## Building & Running

### Prerequisites

```bash
# Add Rust targets for Pico boards
rustup target add thumbv6m-none-eabi           # Pico 1 (ARM)
rustup target add thumbv8m.main-none-eabihf    # Pico 2 (ARM)
```

### Quick Start

```bash
# New project template
# https://github.com/CarlKCarlK/device-envoy-blinky

# Run examples using convenient aliases
cargo blinky                # Simple LED blinky (Pico 1)
cargo blinky-2              # Simple LED blinky (Pico 2)

cargo clock-lcd-w           # LCD clock with WiFi (Pico 1 WiFi)
cargo clock-lcd-2w          # LCD clock with WiFi (Pico 2 WiFi)

cargo clock-led12x4-w       # LED panel clock (Pico 1 WiFi)
cargo clock-led12x4-2w      # LED panel clock (Pico 2 WiFi)

# Check without running (faster builds)
cargo blinky-check          # Compile only
cargo clock-lcd-w-check     # Check Pico 1 WiFi version

# Build and check everything
cargo check-all
```

**Tools:**

- `just` - Optional command runner (install with `cargo install just` or your package manager). See `justfile` for commands.
- `xtask` - Project's custom automation tool (built-in, use via `cargo xtask --help`)

See `.cargo/config.toml` for all cargo aliases.

## Hardware Notes

### Standard Pinouts

Examples use conventional pin assignments for consistency:

- **PIN_0**: LED strip (8-pixel simple example)
- **PIN_1**: Single LED (blinky patterns) - Built-in LEDs are modeled as active-high (OnLevel::High) on all supported boards
- **PIN_3**: LED panel (12×4, 48 pixels)
- **PIN_4**: Extended LED panel (12×8, 96 pixels)
- **PIN_5**: Long LED strip (160 pixels, broadway/marquee effects)
- **PIN_6**: Large LED panel (16×16, 256 pixels)
- **PIN_8**: I²S audio output data pin (`DIN`)
- **PIN_9**: I²S audio output bit clock pin (`BCLK`)
- **PIN_10**: I²S audio output word select pin (`LRC` / `LRCLK`)
- **PIN_13**: Button (active-low)
- **PIN_11, PIN_12**: Servo signals

## Testing

Host-side tests run on your development machine without hardware:

```bash
just check-all
```

`just` is the optional command runner (install with `cargo install just` or your package manager). See **Tools** above.

Tests include:

- LED text rendering comparisons against reference images
- 2D LED matrix mapping algebra
- LED color space conversions

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.

## License

Licensed under either:

- MIT license (see LICENSE-MIT file)
- Apache License, Version 2.0

at your option.
