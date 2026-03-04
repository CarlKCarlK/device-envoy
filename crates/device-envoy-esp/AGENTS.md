# Coding Notes for Agents (ESP32)

Shared rules are in the root [`AGENTS.md`](../../AGENTS.md). This file contains rules specific to the `device-envoy-esp` crate.

- While the crate version remains `0.0.1-alpha`, we do not care about breaking changes. Optimize for the best API design.
- For ESP32 programs that should run forever, use `core::future::pending().await` instead of a timer loop.
- **Hide boilerplate in doctests**: In addition to the shared rules, hide `use esp_backtrace as _`. **Important:** Do NOT hide imports from `device_envoy_esp`, `embassy_time::Duration`, or `smart_leds` because they are unusual and users need to see them to understand what to import.
- Always run `cargo check` before handing work back.
- For `cargo` aliases that target `riscv32imac-unknown-none-elf`, include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

## Generated Files

For this crate, generation is wired through `xtask` for: `audio_player_generated` and `audio_clip_generated`.

## main/inner_main Pattern

Use the `main`/`inner_main` split to allow the `?` operator in example and demo code:

```rust
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    match inner_main(spawner).await {
        Ok(infallible) => match infallible {},
        Err(e) => panic!("{e:?}"),
    }
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<core::convert::Infallible> {
    init_and_start!(p);
    // ... use ? freely here ...
    core::future::pending().await
}
```

This pattern keeps `main` free of `?` (which `-> !` forbids) while keeping `inner_main` ergonomic.

## init_and_start! Macro

Use `init_and_start!(p)` as the **first statement** inside `#[esp_rtos::main]` (or `inner_main`). It:

1. Calls `esp_hal::init(Config::default())` and binds the result to `p`.
2. Starts the Embassy time driver by consuming `p.TIMG0` and `p.SW_INTERRUPT`.

After the macro, every other peripheral is accessible via `p`:

```rust
init_and_start!(p);
let rmt = esp_hal::rmt::Rmt::new(p.RMT, esp_hal::time::Rate::from_mhz(80)).unwrap();
```

**Do not** call `esp_hal::init` or `esp_rtos::start` manually — `init_and_start!` is the canonical way.

For the board peripherals handle, always use `init_and_start!(p)` so `p` is the consistent name across examples.

Optional keyword outputs:

- RMT handle: `init_and_start!(p, rmt80: rmt80, mode: rmt_mode::Blocking|Async)`
- LEDC handle with APB slow clock: `init_and_start!(p, ledc: ledc)`

## Variable Naming Conventions (ESP-specific)

**Type-based naming:**

- `LedStrip` → `led_strip`
- `SosStrip` → `sos_strip`

## Colors

For RGB8 colors, use the predefined constants from `device_envoy_esp::led_strip::colors` rather than creating RGB values manually:

✅ Good:

```rust
use device_envoy_esp::led_strip::colors;
let frame = Frame1d([colors::RED]);
```

❌ Bad:

```rust
use device_envoy_esp::led_strip::RGB8;
let red = RGB8 { r: 255, g: 0, b: 0 };
```

Common colors available: `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`, etc.

## Device/Static Pair Pattern (ESP-specific)

**Multi-instance devices** require passing `&TypeStatic` as the **first** argument when implementing or calling `Type::new`, named `<type>_static`.

Example:

```rust
static SOS_STRIP_STATIC: SosStripStatic = SosStrip::new_static();
let sos_strip = SosStrip::new(&SOS_STRIP_STATIC, channel, spawner)?;
```

## Porting Scope (ESP-specific)

- When porting a device abstraction from another platform crate, port all user-facing sibling submodules/helpers that belong to that abstraction (for example `button` and `button_watch`) instead of adding example-local replacements.
- For LEDC timer/channel resources, follow the crate ownership-claim protocol used by servo abstractions. Do not bypass this protocol with direct ad-hoc LEDC timer/channel setup in examples or device modules.
- If a macro/device abstraction claims an LEDC timer or channel, treat that claim as exclusive for the entire binary (no sharing), and let duplicate usage fail at link time.

## LED Hardware Configuration

Pin assignments and portability guardrails are documented in `README.md` under `Default Pin Assignments`.
Treat that README section as the single source of truth for examples and docs.

## Documentation (ESP-specific)

- In examples, keep `use` statements limited to `device_envoy_esp::...` items; refer to other crates/modules with fully qualified paths inline.
- Use `cargo run --bin <name> --target riscv32imac-unknown-none-elf` as the standard way to run demos/examples; only use short alias commands when they are defined in `.cargo/config.toml`.
- **API completeness**: When linking back to the primary struct example, use phrasing like `See the [LedStrip struct example](Self) for usage.`

## Visibility and Documentation (ESP-specific)

One common `#[doc(hidden)]` + `pub` case in this crate:

```rust
#[doc(hidden)]
pub use esp_hal;  // Used by init_and_start! expansion in downstream crates
```
