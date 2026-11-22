# device-envoy

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)

**Easier Rust applications for ESP32 and Pico.**

Spend your time on the fun part. Device Envoy helps your Rust program animate LED strips, draw on touchscreens, play sounds, join WiFi, and more. You don't have to write interrupt handlers, DMA setup, or background tasks. Each piece of hardware has a few simple methods. For example, `led_strip.animate(frames)` returns right away while the animation keeps running.

**Try it:** [run the Cheap Yellow Display app in your browser](https://carlkcarlk.github.io/device-envoy-cyd-starter/), or start from a template: [Pico blinky](https://github.com/CarlKCarlK/device-envoy-rp-blinky), [ESP32 blinky](https://github.com/CarlKCarlK/device-envoy-esp-blinky), or [Cheap Yellow Display starter](https://github.com/CarlKCarlK/device-envoy-cyd-starter).

Device Envoy supports the Raspberry Pi Pico 1 and Pico 2 via `device-envoy-rp`, and ESP32 boards via `device-envoy-esp` (supported chips: ESP32, ESP32-C2, ESP32-C3, ESP32-C5, ESP32-C6, ESP32-C61, ESP32-H2, ESP32-S2, ESP32-S3).

For CYD hardware and development implementation choices, see the [CYD overview](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/#implementations-1), including ESP32 hardware, Raspberry Pi Pico hardware, interactive browser simulation, and native desktop testing.

**Feedback**: If you try this crate, I’d love to hear how it goes, whether it works well, fails to build, needs clearer docs, or does not fit your hardware. Please send feedback to carlk AT msn.com.

## Example: Animated LED Strip (from RP crate)

This example is from `device-envoy-rp` and cycles a 96-LED strip through red, green, and blue frames.

![Animated 96-LED strip example (APNG)](https://raw.githubusercontent.com/CarlKCarlK/device-envoy/main/crates/device-envoy-core/docs/assets/led_strip_animated.png)

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

    let frame_duration = embassy_time::Duration::from_millis(300);
    led_strip_animated.animate([
        (Frame1d::filled(colors::RED), frame_duration),
        (Frame1d::filled(colors::GREEN), frame_duration),
        (Frame1d::filled(colors::BLUE), frame_duration),
    ]);

    core::future::pending().await
}
```

## Workspace Crates

- `crates/device-envoy`: top-level landing crate [![crates.io - device-envoy](https://img.shields.io/crates/v/device-envoy?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy) [![docs.rs - device-envoy](https://img.shields.io/docsrs/device-envoy?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy)
- `crates/device-envoy-rp`: Raspberry Pi Pico focused crate [![crates.io - rp](https://img.shields.io/crates/v/device-envoy-rp?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy-rp) [![docs.rs - rp](https://img.shields.io/docsrs/device-envoy-rp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-rp)
- `crates/device-envoy-esp`: ESP crate (ESP32, ESP32-C2, ESP32-C3, ESP32-C5, ESP32-C6, ESP32-C61, ESP32-H2, ESP32-S2, ESP32-S3) [![crates.io - esp](https://img.shields.io/crates/v/device-envoy-esp?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy-esp) [![docs.rs - esp](https://img.shields.io/docsrs/device-envoy-esp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-esp)
- `crates/device-envoy-core`: shared core APIs used across platform crates [![crates.io - core](https://img.shields.io/crates/v/device-envoy-core?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy-core) [![docs.rs - core](https://img.shields.io/docsrs/device-envoy-core?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-core)

## Features

- CYD 320×240 display and touch support with calibration, orientation, memory-efficient tiled rendering, and an optional browser simulator
- LED strips and panels for NeoPixel-style (WS2812) hardware
- WiFi auto-connect and credential management
- Audio player over I2S
- Button input and debouncing
- Servo control and animation
- Flash storage for persistent configuration
- LCD text display support
- IR remote decoding
- RFID card reading
- Clock synchronization helpers
- 4-digit seven-segment display control
- Single LED control and animation

## Forum

- **[Using Embassy to build applications](https://github.com/CarlKCarlK/device-envoy/discussions)**

  A place to talk about writing embedded applications with Embassy: sharing code, asking practical questions, and learning what works in practice.

## Videos and Articles

- [device-envoy: Making Embedded Fun with Rust, Embassy, and Composable Device Abstractions](https://medium.com/@carlmkadie/device-envoy-esp-making-embedded-esp32-fun-872e251b88f3) -- versions: [Pico article](https://medium.com/@carlmkadie/device-envoy-making-embedded-fun-31534917414b), [ESP32 article](https://medium.com/@carlmkadie/device-envoy-esp-making-embedded-esp32-fun-872e251b88f3) or [video](https://www.youtube.com/watch?v=iUu6hvJLVOU)
- [How Rust & Embassy Shine on Embedded Devices](https://medium.com/@carlmkadie/how-rust-embassy-shine-on-embedded-devices-part-1-9f4911c92007) by Carl M. Kadie and Brad Gibson
- [Nine Rules for Compile-Time Work with Rust `const fn`](https://medium.com/gitconnected/nine-rules-for-compile-time-work-with-rust-const-fn-part-1-a29f7dd62b2f), published in *Level Up Coding*, includes examples from Linkage Blaze and Device Envoy
- [More Rust articles](https://medium.com/@carlmkadie)

## Thanks

Special thanks to [Brad Gibson](https://github.com/U007D/), organizer of the [Seattle Rust User Group](https://www.meetup.com/seattle-rust-meetup/). He introduced me to Rust programming on microcontrollers, suggested the term *device abstraction*, and encouraged thinking in terms of shared traits across controller families. Those conversations helped set the goals for device-envoy.

## Policy on AI-assisted development and contributions

The use of AI tools is permitted for development and contributions to this repository. AI may be used as a productivity aid for drafting, exploration, and refactoring.

All code and documentation contributed to this repository must be reviewed, edited, and validated by a human contributor. AI tools are not a substitute for design judgment, testing, or responsibility for correctness.

[AGENTS.md](AGENTS.md) contains the general instructions and constraints given to AI tools used during development of this repository.

## Development Guide

If you want to edit this workspace, start here: [development guide](docs/development.md).

## License

Licensed under either:

- MIT license (see LICENSE-MIT)
- Apache License, Version 2.0 (see LICENSE-APACHE)

at your option.
