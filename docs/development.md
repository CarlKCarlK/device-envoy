# Development Guide

This guide is for editing the `device-envoy` workspace itself.

If you only want to use `device-envoy-rp` or `device-envoy-esp` in your own project, start from a template:

- [`device-envoy-rp-blinky`](https://github.com/CarlKCarlK/device-envoy-rp-blinky)
- [`device-envoy-esp-blinky`](https://github.com/CarlKCarlK/device-envoy-esp-blinky)

## Toolchains and Targets

You may need all embedded targets below, depending on what you build:

- RP Pico 1 (ARM): `thumbv6m-none-eabi`
- RP Pico 2 (ARM): `thumbv8m.main-none-eabihf`
- ESP32-C5 / ESP32-C6 / ESP32-C61 / ESP32-H2 (RISC-V): `riscv32imac-unknown-none-elf`
- ESP32-C2 / ESP32-C3 (RISC-V): `riscv32imc-unknown-none-elf`
- ESP32 / ESP32-S2 / ESP32-S3 (Xtensa):
  `xtensa-esp32-none-elf`, `xtensa-esp32s2-none-elf`, `xtensa-esp32s3-none-elf`

Install Rust targets:

```bash
rustup target add thumbv6m-none-eabi
rustup target add thumbv8m.main-none-eabihf
rustup target add riscv32imc-unknown-none-elf
rustup target add riscv32imac-unknown-none-elf
```

Xtensa ESP chips (ESP32 / ESP32-S2 / ESP32-S3) require the ESP Rust toolchain/runtime flow used by this repo:

- Install/use the `+esp` toolchain as needed for Xtensa builds.
- Load your ESP environment before S3 commands (for this repo that is usually `source "$HOME/export-esp.sh"`).
- Use `just` recipes for S3 run/check commands in `crates/device-envoy-examples-esp/justfile`.

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
cargo check-all
```

Equivalent command:

```bash
cargo run --manifest-path xtask/Cargo.toml -- check-all
```

## Generate Docs

From the workspace root:

```bash
just docs
```

This is the only authoritative documentation build. It rebuilds Core with the
`host`, `wasm`, and `doc-images` features, rebuilds ESP and RP documentation,
stages the shared images, and verifies sentinel pages and images in the final
snapshot. A successful command leaves all three reviewable output trees in
`target/`.

The explicitly partial `docs-core-only`, `docs-rp-only`, and `docs-esp-only`
recipes are useful for focused iteration, but their output is not the complete
review snapshot. The `*-no-images`, `*-unvalidated`, and `docs-incomplete`
recipes are faster, incomplete previews. After any partial build, rerun
`just docs` before review or publishing.

Optional: build and open docs in a browser:

```bash
just show-docs
```

The platform-specific `show-docs-rp`, `show-docs-esp`, and `show-docs-core`
recipes open partial previews instead.

## Declaration Macros

All 33 declaration macros (`led_strip!`, `ir_mapping!`, `servo!`, and so on)
are [`macro-schema`](https://github.com/CarlKCarlK/macro-schema) `define!`
schemas, each with a `generate { ... }` template. The framework parses,
validates, defaults, and documents every call; there are no hand-written
matchers. The template language is documented on `define!` in
`macro-schema`'s rustdoc, and the framework's design is in its
`docs/architecture.md`. The `0.2.0` section of [CHANGELOG.md](../CHANGELOG.md)
is the migration guide from the old syntax.

### Where schemas live

- Each schema sits in the module whose API it generates, next to the runtime
  code its template calls. One family is one file.
- RP and ESP keep separate schemas and templates, with no shared macro code.
  Each chip's template is that chip's API in its HAL's terms. Logic both chips
  need is shared through `device-envoy-core` as ordinary Rust, as it is for
  `LedStrip`, `Frame1d`, the servo loop, the button-watch runtime, and the I2C
  task loop.

### How templates are organized

The pattern follows the hardware:

- **A single device as a one-member group** (RP). RP devices in a group share
  a PIO block, so RP `led_strip!`, `led2d!`, `ir!`, `ir_kepler!`,
  `ir_mapping!`, and both chips' `lcd_text!` expand to a hidden one-member
  group and add a `new` that unpacks it.
- **A group that calls its single** (ESP). Each ESP IR receiver owns its own
  RMT channel and task, so `irs!`, `ir_keplers!`, and `ir_mappings!` invoke the
  single macro per member and add a group constructor.
- **Direct.** `audio_player!`, `button_watch!`, `led!`, `servo!`,
  `servo_player!`, `i2cs!`, and the clip macros emit their items directly.
- **Choice by type, not by spelling.** A template cannot branch on what a
  value is. When output depends on a choice, make the choice a type and put
  the differences in trait impls. ESP `led_strip!`'s `engine` is the type `Rmt`
  or `Spi`, and the `LedEngine` and `EngineResource` impls carry the driver
  type, buffer length, and constructor resource.
- **A group that calls a hidden declaration macro.** RP `led_strips!` emits
  each strip through the hidden `__led_strip_type!` schema, under the member's
  name or, behind a panel, a hidden `<Name>LedStrip` name.

The hand-written `macro_rules!` that remain are not declarations:
`combine!`, `tone!`, `tga!`, and `init_and_start!` take positional or
statement-shaped input. The rest generate repetitive impls inside the crate
(`__define_ir_task!`, `impl_wifi_pio!`, `servo_pin_map!`,
`impl_dma_irq_map_all_irqs!`, `__cyd_tga!`, `__impl_wifi_auto_fields!`).

### Rules for generated code

- **Constructor argument order**, from most specific to least: the device's
  own pins, then its peripherals or channels, then its configuration data
  (such as `on_level` or an IR `button_map`), then shared resources, then the
  spawner. Schema fields are listed in the same order. A group declaration
  keeps its shared field first; a group constructor takes each member's
  arguments first and the shared resource last. Hand-written constructors
  follow the rule too, with a device's own `&'static …Static` first (see the
  Device/Static Pair Pattern in `AGENTS.md`). The four CYD constructors are an
  accepted exception, kept to avoid breaking about 300 call sites.
- **Visibility.** Generated methods are `pub`. The declared type carries the
  user's visibility, and members inherit the group's.
- **No extra user dependencies.** Generated code reaches `static_cell`,
  `heapless`, and `embassy_sync` through hidden `$crate::__*` re-exports, so
  users do not add them to their own `Cargo.toml`.
- **Doc links name the platform crate.** In generated code, `crate::` is the
  user's crate. Write `device_envoy_rp::…` or `device_envoy_esp::…`.
- **Everything generated is documented**, so a user crate that denies
  `missing_docs` and warnings builds.
- **Repeated field docs stay inline.** For example, the RP note "Each PIO
  serves one device or group…" appears on every macro page that takes `pio`,
  because readers should not have to follow a link. When editing it, change
  every copy in one commit.

### Intentional differences

These look inconsistent but are deliberate:

- `max_current` defaults to 250 mA for one device, but each member of a
  `led_strips!` group must set it: currents on a shared supply add up.
- `dma` is defaulted on RP `audio_player!` but required on ESP. No default
  fits every ESP chip (the ESP32 and ESP32-S2 use `DMA_I2S0`, newer chips
  `DMA_CH0`).
- `pcm_clip!` requires a source sample rate; `adpcm_clip!` reads it from the
  WAV header. A raw PCM file has no header.
- Animations are sized by `max_frames` on LED strips and `max_steps` on `led!`
  and `servo_player!`.
- `servo!`'s `new` is infallible on RP and returns `Result` on ESP, where LEDC
  setup can fail.
- Other RP/ESP differences come from the HALs: which resources a constructor
  takes, `impl Into<Peri<..>>` on RP versus by-value peripherals on ESP, and
  servo setup types.

### Tests that guard the macros

`cargo check-all` runs all of these. Each was confirmed to fail on the bug it
guards against.

- `crates/device-envoy-core/tests/macro_expansion.rs` expands RP, ESP, and Core
  schemas on the host with `macro_schema_core::expand_definition`, without
  compiling the HALs. It also holds the parity tests:
  `rp_and_esp_macros_document_the_same_contract` compares the Syntax and Fields
  docs, and `rp_and_esp_samples_generate_the_same_api` compares each generated
  type's trait impls, methods, return types, and constants. Intentional
  differences are on explicit allow-lists. `every_macro_page_leads_to_usage_examples`
  checks that every public macro page shows or links to an example.
- `crates/device-envoy-core/tests/doc_conventions.rs` checks that macro
  summaries match across chips, that Core item docs link instead of naming
  "the platform crate", that the `led_strip` re-export list is complete, and
  the link conventions in `AGENTS.md`.
- `crates/device-envoy-rp/tests-compile-only/generated_code_lints.rs` and
  `crates/device-envoy-esp/tests/embedded/generated_code_lints_compile.rs`
  build generated code in a crate that denies warnings and `missing_docs`.

`macro_expansion.rs` and `doc_conventions.rs` read the RP and ESP sources, so
they are excluded from the published Core package.

### Rustdoc pitfalls

- A `///` comment attaches to the next item even across blank lines. Put a
  macro's hand-written docs inside its `define!`, or they land on a hidden
  item.
- Rustdoc shows a re-export's own doc comment before the item's docs, but only
  for a named re-export. A glob re-export drops it, so the `led_strip`
  re-exports are named.

### Deferred

- Folding Kepler IR into IR mapping: a Kepler receiver is a mapping receiver
  with a fixed 21-entry table, and the templates copy the mapping code. Doing
  it on RP alone would add an API difference between the chips; doing it on
  both needs a hidden shared schema to save about 30 lines. Revisit if the IR
  logic changes.
- `pio` 0.4: the `proc-macro-error2` future-incompatibility warning comes from
  `pio-proc` 0.3 through `pio` 0.3. Move to `pio` 0.4 when embassy-rp does.

## Release Process

The canonical release procedure, including PR/CI gates, starter validation,
publishing order, downstream updates, and tagging, is:

- [`release_checklist.md`](release_checklist.md)

Keep release steps there rather than duplicating them in this development
guide.

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

ESP defaults are chip- and board-dependent.

For current source-of-truth pin/capability mappings, see:

- `crates/device-envoy-examples-esp/xtask/src/boards.rs` (`BOARD_PROFILES`)
- `crates/device-envoy-examples-esp/xtask/src/main.rs` (capability-based example/test gating)
- Generated board examples under `crates/device-envoy-examples-esp/examples/<chip>/<board>/`
