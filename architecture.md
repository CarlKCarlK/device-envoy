# Architecture

This project supports four hardware targets:

1. Pico 1 (RP2040)
2. Pico 2 (RP2350, ARM core)
3. ESP32-C6
4. ESP32-S3

At a CPU-architecture level, those targets map to three CPU architectures used in this repo:

1. ARM Cortex-M (`thumbv6m`, `thumbv8m.main`)
2. RISC-V (`riscv32imac`)
3. Xtensa (`xtensa-esp32s3`)

## Three Crates and Their Roles

The workspace is split into three primary library crates:

1. `crates/device-envoy-core`
2. `crates/device-envoy-rp`
3. `crates/device-envoy-esp`

`device-envoy-core` contains platform-independent logic.

`device-envoy-rp` contains Pico-specific constructors, peripherals, and wiring to RP hardware (`PIO`, `DMA`, RP pins, RP interrupts).

`device-envoy-esp` contains ESP32-specific constructors, peripherals, and wiring to ESP hardware (`RMT`, `SPI`, `I2S`, LEDC, ESP Wi-Fi peripherals).

The top-level workspace declaration reflects this split in [Cargo.toml](/home/carlk/programs/device-envoy/Cargo.toml).

## Conditional Compilation

The codebase uses conditional compilation in two layers.

1. Cargo target and target-architecture selection
2. Feature flags and `#[cfg(...)]` inside modules

### RP crate (`device-envoy-rp`)

`device-envoy-rp` enforces board/arch combinations at compile time with `compile_error!` guards (for example, `pico1` vs `pico2`, `arm` vs `riscv`) in [crates/device-envoy-rp/src/lib.rs](/home/carlk/programs/device-envoy/crates/device-envoy-rp/src/lib.rs).

### ESP crate (`device-envoy-esp`)

`device-envoy-esp` selects chip support through target-architecture-specific dependencies in [crates/device-envoy-esp/Cargo.toml](/home/carlk/programs/device-envoy/crates/device-envoy-esp/Cargo.toml):

1. `target_arch = "riscv32"` uses `esp32c6`
2. `target_arch = "xtensa"` uses `esp32s3`

Its xtask builds both required targets (`riscv32imac-unknown-none-elf` and `xtensa-esp32s3-none-elf`) in [crates/device-envoy-esp/xtask/src/main.rs](/home/carlk/programs/device-envoy/crates/device-envoy-esp/xtask/src/main.rs).

## How We Know Resource Differences

### Pico 2 has more PIO than Pico 1

This is encoded directly in RP crate docs and code:

1. The RP crate glossary says Pico 1 has 2 PIO resources and Pico 2 has 3 in [crates/device-envoy-rp/src/lib.rs](/home/carlk/programs/device-envoy/crates/device-envoy-rp/src/lib.rs).
2. `PIO2` IRQ mapping is compiled only for `feature = "pico2"` in [crates/device-envoy-rp/src/pio_irqs.rs](/home/carlk/programs/device-envoy/crates/device-envoy-rp/src/pio_irqs.rs).

### ESP32-S3 has more I2S peripherals than ESP32-C6

This comes from chip-specific PAC definitions selected by `esp-hal`:

1. ESP32-C6 PAC exposes `I2S0` in `Peripherals` (no `I2S1` field).
2. ESP32-S3 PAC exposes both `I2S0` and `I2S1` in `Peripherals`.

In other words, the type system reflects chip capability. If a peripheral field does not exist for a chip, code using it does not compile for that target.

## Why Separate Constructors From the Rest of the API

We intentionally separate platform-specific construction from platform-independent behavior.

Platform-specific parts:

1. Constructor signatures
2. Peripheral ownership types
3. Pin/channel selection

Platform-independent parts:

1. Data types
2. Events and state machines
3. Formatting, parsing, shared algorithms

This keeps APIs consistent for users while allowing hardware-specific setup per target.

## `led_strip` Construction Examples

### 1) Pico 1 / Pico 2 construction (`PIO` + `DMA`)

```rust
let led_strip = LedStripType::new(
    p.PIN_0,
    p.PIO0,
    p.DMA_CH0,
    spawner,
)?;
```

RP construction depends on PIO and DMA resources, which are central to Pico LED driving.

### 2) ESP32 construction in two ways

RMT path:

```rust
init_and_start!(p, rmt80, rmt_mode::Blocking);
let led_strip = LedStripType::new(p.GPIO10, rmt80.channel0, spawner)?;
```

SPI path:

```rust
use device_envoy_esp::{
    init_and_start, led_strip,
    led_strip::{Current, Engine},
};

led_strip! {
    LedStrip8Spi {
        len: 8,
        max_current: Current::Milliamps(180),
        engine: Engine::Spi,
        max_frames: 2,
    }
}

init_and_start!(p);
let led_strip8_spi = LedStrip8Spi::new(p.GPIO10, p.SPI2, spawner)?;
```

ESP examples demonstrate both RMT-driven and SPI-driven LED strip construction.

## Why We Do Not Merge Pico and ESP Constructors

We do not merge RP and ESP constructors into a single constructor API because their hardware models differ too much.

1. RP uses `PIO` + `DMA` ownership and IRQ mapping.
2. ESP uses `RMT` channels or `SPI` peripherals with different channel/clock models.

A forced "one-size-fits-all" constructor would be less clear, less type-safe, and harder to use correctly than platform-native constructors with shared behavior behind them.

## Presentation to User

The plan is that users primarily interact with:

- `device-envoy-rp` — Pico-focused API and docs
- `device-envoy-esp` — ESP32-focused API and docs

Internally, both crates depend on:

- `device-envoy-core` — shared types and implementation
