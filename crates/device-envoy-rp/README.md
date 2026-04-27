# device-envoy-rp

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)
[![crates.io](https://img.shields.io/crates/v/device-envoy-rp?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy-rp)
[![docs.rs](https://img.shields.io/docsrs/device-envoy-rp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-rp)

**Build Pico applications with LED panels, easy WiFi, and composable device abstractions.**

`device-envoy-rp` is a library for building embedded applications in Rust, built on the Embassy framework. It organizes hardware around *device abstractions*.

A device abstraction is a software encapsulation of hardware that manages timing, tasks, control flow, interrupts, channels, and state within the abstraction.

Rather than replacing HALs or drivers, `device-envoy-rp` builds on them. It defines device abstractions that expose a small set of simple operations to the rest of the program.

`device-envoy-rp` supports the Raspberry Pi Pico and Pico 2 (ARM cores). RISC-V core support on the Pico 2 is not currently included. Its sibling package, [`device-envoy-esp`](https://docs.rs/device-envoy-esp), supports all ESP32 families currently supported by [esp-hal](https://github.com/esp-rs/esp-hal).

## Start From a Template

Want a minimal starting project?

- [`device-envoy-rp-blinky` on GitHub](https://github.com/CarlKCarlK/device-envoy-rp-blinky)

## Status

⚠️ **Alpha / Experimental**

The API is actively evolving. Not recommended for production use, but excellent for experimentation, learning, and exploratory projects.

## Features

- **[LED Strips](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/led_strip/) & [Panels](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/led2d/)**  - NeoPixel-style (WS2812) LED arrays with 2D text rendering, animation, embedded-graphics support. Provides efficient options for power limiting and color correction.
- **[WiFi (Pico W)](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/wifi_auto/)** - Connect to the Internet with automatic credentials management. On boot, opens a web form if WiFi credentials aren't saved, then connects seamlessly to a stored network. Requires Pico W; WiFi is not supported on non-W boards.
- **[Audio Player](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/audio_player/)** - Play audio clips over I²S hardware with runtime sequencing, volume control, and compression.
- **[Button Input](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/button/)** - Button handling with debouncing
- **[Servo Control](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/servo/)** - Servo positioning and animation
- **[Flash Storage](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/flash_block/)** - Type-safe, on-board persistent storage
- **[LCD Display](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/lcd_text/)** - Text display (HD44780)
- **[IR Remote](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/ir/)** - Remote control decoder (NEC protocol)
- **[RFID Reader](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/rfid/)** - Card detection and reading (MFRC522)
- **[Clock Sync](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/clock_sync/)** - Network time synchronization utilities
- **[LED4 Display](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/led4/)** - 4-digit, 7-segment LED display control with optional animation and blinking
- **[Single LED](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/led/)** - Single LED control with animation support

## Forum

- **[Using Embassy to build applications](https://github.com/CarlKCarlK/device-envoy/discussions)**  
  A place to talk about writing embedded applications with Embassy: sharing code, asking practical questions, and learning what works in practice. Not limited to Pico or ESP boards, or to `device-envoy`.

## Videos and Articles

- [device-envoy: Making Embedded Pico Fun with Rust, Embassy, and Composable Device Abstractions](https://medium.com/@carlmkadie/device-envoy-making-embedded-fun-31534917414b) -- versions: [article](https://medium.com/@carlmkadie/device-envoy-making-embedded-fun-31534917414b) or [video](https://www.youtube.com/watch?v=iUu6hvJLVOU)
- [How Rust & Embassy Shine on Embedded Devices](https://medium.com/@carlmkadie/how-rust-embassy-shine-on-embedded-devices-part-1-9f4911c92007) by Carl M. Kadie and Brad Gibson.
- [More Rust articles](https://medium.com/@carlmkadie)

## Thanks

Special thanks to [Brad Gibson](https://github.com/U007D/), organizer of the [Seattle Rust User Group](https://www.meetup.com/seattle-rust-meetup/). He introduced me to Rust programming on microcontrollers, suggested the term *device abstraction*, and encouraged thinking in terms of shared traits across controller families. Those conversations helped set the goals for device-envoy.

## Examples & Demos

The project includes **examples** (single-device tests) in `examples/` and **demo applications** in `demos/` showing integration patterns:

### Example: animated LED strip

This example cycles a 96-LED strip through red, green, and blue frames.

![Animated 96-LED strip example (APNG)](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/crates/device-envoy-rp/docs/assets/led_strip_animated.png)

It shows how device-envoy generates a struct (device abstraction) for an LED strip and then animates a sequence of frames.

```rust,no_run
# #![no_std]
# #![no_main]
# use panic_probe as _;
# use core::convert::Infallible;
use device_envoy_rp::{Result, led_strip::{LedStrip as _, Frame1d, colors}};
use device_envoy_rp::led_strip;

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
    ]);

    core::future::pending().await // run forever
}
```

> For complete, runnable examples (including wiring and setup), see the `examples/` and `demos/` directories.

- **Basic LED Examples**: Simple on/off control with blinky pattern
- **LED Strip Examples**: Simple animations, color control, text rendering

- **LED Panel Examples**: 12×4, 12×8, and multi-panel configurations with graphics

![Animated LED panel Go Go example](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/crates/device-envoy-rp/docs/assets/led2d2.png)

- **Button Examples**: Debouncing and state handling
- **Servo Examples**: Position sweeps and animation playback
- **WiFi Examples**: WiFi setup, time sync, DNS
- **Flash Examples**: Configuration persistence and data reset

See the `examples/` and `demos/` directories for complete runnable code.

## Building & Running

- If you just want to use this library, start from the template project: [`device-envoy-rp-blinky`](https://github.com/CarlKCarlK/device-envoy-rp-blinky).
- If you want to edit this project, see the [Development Guide](docs/development_guide/index.html).

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](https://github.com/CarlKCarlK/device-envoy/blob/main/AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.

## License

Licensed under either:

- MIT license (see the repository root `LICENSE-MIT` file)
- Apache License, Version 2.0 (see the repository root `LICENSE-APACHE` file)

at your option.
