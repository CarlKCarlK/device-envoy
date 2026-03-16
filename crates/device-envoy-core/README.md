# device-envoy-core

[![GitHub](https://img.shields.io/badge/github-device--envoy-8da0cb?style=flat&labelColor=555555&logo=github)](https://github.com/CarlKCarlK/device-envoy)
[![workspace-only](https://img.shields.io/badge/workspace-internal-999999?style=flat&labelColor=555555)](https://github.com/CarlKCarlK/device-envoy/tree/main/crates/device-envoy-core)
[![docs.rs device-envoy-rp](https://img.shields.io/docsrs/device-envoy-rp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/)
[![docs.rs device-envoy-esp](https://img.shields.io/docsrs/device-envoy-esp?style=flat&color=66c2a5&labelColor=555555)](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/)

**Shared traits and data types for the device-envoy workspace.**

`device-envoy-core` holds platform-agnostic APIs used by:

- [`device-envoy-rp`](https://docs.rs/device-envoy-rp/latest/device_envoy_rp/) (Raspberry Pi Pico targets)
- [`device-envoy-esp`](https://docs.rs/device-envoy-esp/latest/device_envoy_esp/) (ESP32 targets)

For most users, start with one of those platform crates. Their docs include constructors, board-specific setup, and re-exported trait APIs from `device-envoy-core`.

## Status

⚠️ **Alpha / Experimental**

APIs are actively evolving.

## License

Licensed under either:

- MIT license (see `LICENSE-MIT`)
- Apache License, Version 2.0

at your option.
