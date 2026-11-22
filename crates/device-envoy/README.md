# device-envoy

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)
[![crates.io](https://img.shields.io/crates/v/device-envoy?style=flat&color=fc8d62&logo=rust)](https://crates.io/crates/device-envoy)
[![docs.rs](https://img.shields.io/docsrs/device-envoy?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy)

**Easier Rust applications for ESP32 and Pico.**

Device Envoy helps your Rust program animate LED strips, draw on touchscreens, play sounds, join WiFi, and more. You don't have to write interrupt handlers, DMA setup, or background tasks.

This `device-envoy` crate is a starting point that helps you pick the crate for your board.

Do not start a new project with `device-envoy` itself. Choose the crate for your board below instead.

For CYD implementation choices, see the [canonical overview](https://docs.rs/device-envoy-core/latest/device_envoy_core/cyd/#implementation-overview), which distinguishes hardware, interactive browser simulation, and native desktop testing.



- [`device-envoy-rp`](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/) for Raspberry Pi Pico targets
- [`device-envoy-esp`](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/) for ESP32 targets

## License

Licensed under either:

- MIT license (see the repository root `LICENSE-MIT` file)
- Apache License, Version 2.0 (see the repository root `LICENSE-APACHE` file)

at your option.
