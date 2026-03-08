# Coding Notes for Agents (RP / Pico)

Shared rules are in the root [`AGENTS.md`](../../AGENTS.md). This file contains rules specific to the `device-envoy-rp` crate.

- While the crate version remains `0.0.3-alpha`, we do not care about breaking changes. Optimize for the best API design.
- For Pico programs that should run forever, use `core::future::pending().await` instead of a timer loop.
- **Hide boilerplate in doctests**: In addition to the shared rules, hide `use panic_probe as _` and `use defmt_rtt as _`. **Important:** Do NOT hide imports from `device_envoy_rp`, `embassy_time::Duration`, `smart_leds`, or `embedded_graphics` because they are unusual and users need to see them to understand what to import.
- Always run `cargo check-all` before handing work back; xtask keeps doctests and examples in sync.
- Do not add redundant `just` recipes that only mirror an existing `cargo` alias/command. If the behavior is the same, keep only the `cargo` command.
- For `cargo` aliases that target embedded triples (`thumbv6m-none-eabi`, `thumbv8m.main-none-eabihf`, or `riscv32imac-unknown-none-elf`), include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

## Generated Files

For this crate, generation is wired through `xtask` for: `audio_player_generated`, `audio_clip_generated`, `lcd_text_generated`, and `servo_player_generated`.

## Const-Only APIs

**The `LedLayout` type must remain fully const.** All methods on `LedLayout` must be `const fn`. This enables compile-time LED layout validation and zero-runtime-cost transformations. If you add a method to `LedLayout` that is not `const fn`, report this as an error. The existing doctests enforce const-ness by using methods in const contexts; removing `const` from any method will cause compilation to fail.

## Variable Naming Conventions (RP-specific)

**Type-based naming:**

- `Led12x4` → `led12x4` (dimension suffix)
- `WifiAuto` → `wifi_auto`
- `LedStrip` → `led_strip`
- `Led12x4ClockDisplay` → `led12x4_clock_display`

**When to deviate:**

- Generic/contextual names are acceptable when the type is obvious and verbose naming would be redundant:
  - ✅ `button` (not `button_pico2`) when only one button exists
  - ✅ `clock` (not `clock_0`) when context is clear

**Project-specific patterns:**

- For the board peripherals handle from `embassy_rp::init`, always use the shorthand `let p = embassy_rp::init(...)` so examples stay consistent.

**Reference variables:**

- `led12x4` → `led12x4_ref`
- `wifi_auto` → `wifi_auto_ref`

## Terminology

- **PIO resource** (not "PIO block") — Use "PIO resource" or just "PIO" when referring to the PIO peripheral.

## PIO IRQ Mapping

- Use the shared `crate::pio_irqs::PioIrqMap` trait when a module only needs to map a PIO resource to `Irqs` + `irqs()`.
- If a module needs extra PIO-specific behavior (for example task spawning hooks), define a module-local trait that extends `PioIrqMap` and only add the extra methods there.

## Colors

For RGB8 colors, use the predefined constants from `smart_leds::colors` (re-exported from `led_strip::colors`) rather than creating RGB values manually:

✅ Good:

```rust
use device_envoy_rp::led_strip::colors;
let frame = [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW];
```

❌ Bad:

```rust
use device_envoy_rp::led_strip::Rgb;
let red = Rgb::new(255, 0, 0);
let green = Rgb::new(0, 255, 0);
```

Common colors available: `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`, etc.

When working directly with the `embedded_graphics` crate, using `colors::RED.to_rgb888()` (with `device_envoy_rp::led_strip::ToRgb888` in scope) is acceptable to avoid conversions.

## Device/Static Pair Pattern (RP-specific)

- **Hardware singletons** (e.g., `WifiAuto` — one WiFi chip per device) hide the static inside `Type::new()` using a function-scoped static, so users never see `TypeStatic`.
- **Multi-instance devices** (e.g., `Led4Rp` — can have multiple) require passing `&TypeStatic` as the **first** argument when implementing or calling `Type::new`, named `<type>_static` (e.g., `led4_static: &'static Led4RpStatic`).

Hardware singleton (static hidden inside `new()`):

```rust
// User code - no static needed!
let wifi_auto = WifiAuto::new(
    p.PIN_23,
    p.PIN_25,
    // ... more pins ...
    spawner,
)?;
```

Multi-instance device (static passed as first argument):

```rust
static LED4_STATIC: Led4RpStatic = Led4Rp::new_static();
let led4 = Led4Rp::new(&LED4_STATIC, cells, segments, spawner)?;
```

## Documentation (RP-specific)

- In examples, keep `use` statements limited to `device_envoy_rp::...` items; refer to other crates/modules with fully qualified paths inline.
- Keep example shape consistent: show an async function that receives `Peripherals`/`Spawner` (or other handles) and constructs the device with `new_static`/`new`; avoid mixing inline examples without that pattern next to function-based ones.
- In examples, prefer importing the types you need (`use crate::foo::{Device, DeviceStatic};`) instead of fully-qualified paths for statics.
- Use `cargo run --bin <name> --target <target> --features <features>` as the standard way to run demos/examples; only use short `cargo demo-*` commands when they are defined as aliases in `.cargo/config.toml`.
- **API completeness**: When linking back to the primary struct example, use phrasing like `See the [WifiAuto struct example](Self) for usage.`

### Style Macro Documentation

For style macros (for example `audio_clip!`, `audio_player!`, `led_strip!`), document them with a consistent structure:

1. One-line summary
2. Compact syntax block
3. Inputs (`$vis`, `$name`, etc.) including which are optional
4. Required fields
5. Optional fields/defaults
6. Link to the module documentation for full usage examples

Whenever a style macro implementation or its docs change, verify that macro docs and behavior stay in sync (accepted fields, defaults, optional inputs, generated items, and linked examples).

## LED Hardware Configuration

Examples use the following standard PIO resource and pin assignments:

- **PIO resource 0 + PIN_0** — 8 LEDs in a line (e.g., `led_strip_single.rs`)
- **PIN_3** — 12×4 panel (48 pixels, e.g., `led_strip_3_on_a_pio.rs`)
- **PIN_4** — Two 12×4 panels combined into 12×8 panel (96 pixels)

When writing new examples or documentation, follow this convention for consistency.

### Single LED Wiring (for `Led` device)

For single LED examples using the `Led` device abstraction, use **PIN_1**. The `Led` device supports both high-level-on and low-level-on configurations:

**High level on (default):**

- LED anode (long leg) → 220Ω resistor → PIN_1
- LED cathode (short leg) → GND
- Use: `Led::new(&led_static, pin, OnLevel::High, spawner)`

**Low level on:**

- LED anode (long leg) → 3.3V
- LED cathode (short leg) → 220Ω resistor → PIN_1
- Use: `Led::new(&led_static, pin, OnLevel::Low, spawner)`

The `OnLevel` enum specifies what pin level turns the LED on.

### Button Pin

The standard button pin across examples is **PIN_13**:

```rust
let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
```

Use this consistently when adding button input to examples.

### Servo Pins

The standard servo pins across examples are **PIN_11** and **PIN_12**.
