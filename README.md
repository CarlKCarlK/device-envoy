# device-envoy workspace

This repository is a Rust workspace for platform-specific crates and a shared core crate.

- `crates/device-envoy-rp`: Raspberry Pi Pico focused crate (from `device-envoy`)
- `crates/device-envoy-esp`: ESP focused crate (from `device-envoy-esp32`, now `device-envoy-esp`)
- `crates/device-envoy-core`: shared crate (currently minimal)

Current migration intent:

- Keep platform crates working as-is while moving toward shared functionality in `device-envoy-core`.

todo000 top level readme
todo000 Thank Brad
