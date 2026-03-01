# Coding Notes for Agents

- While the crate version remains `0.0.1-alpha`, we do not care about breaking changes. Optimize for the best API design.

- When loading data from flash (or any other storage) into a local variable, name the variable after the concrete type. Example: `DeviceConfig` data should live in variables like `device_config`, not generic `config` or `flash0`.
- Avoid introducing `unsafe` blocks. If a change truly requires `unsafe`, call it out explicitly and explain the justification so the user can review it carefully.
- Avoid silent clamping; prefer asserts or typed ranges so out-of-range inputs fail fast.
- Prefer `no_run` doctests; use `ignore` only when absolutely necessary (and call out why). Running doctests is best when possible, but rarely feasible for embedded code.
- Always use `rust,no_run` in doctest fences, not just `no_run`.
- For ESP32 programs that should run forever, use `core::future::pending().await` instead of a timer loop.
- **Hide boilerplate in doctests** using the `#` prefix (e.g., `# #![no_std]`). Hide lines that are noise to the reader but required for compilation: `#![no_std]`, `#![no_main]`, `use esp_backtrace as _`, and standard imports like `use embassy_executor::Spawner;`. Keep only the essential code showing how to use the API. **Important:** Do NOT hide imports from `device_envoy_esp32`, `embassy_time::Duration`, or `smart_leds` because they are unusual and users need to see them to understand what to import.
- When adding docs for modules or public items, link readers to the primary struct and keep the single compilable example on that struct; other items should point back to it rather than duplicating examples.
- Prefer `const` values defined in the local context (inside the function/example) rather than at module scope when they're only used there.
- Always run `cargo check` before handing work back.
- For `cargo` aliases that target embedded triples (`riscv32imac-unknown-none-elf`), include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

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

async fn inner_main(spawner: Spawner) -> device_envoy_esp32::Result<core::convert::Infallible> {
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

## Generated Files

- Treat generated files under `src/**/_generated.rs` as build outputs, not source of truth.
- When changing generated docs/examples, edit the corresponding generator template in `xtask/src/*_generated.rs` first.
- If you must patch a generated file directly for an urgent fix, make the matching template change in the same PR so regeneration does not revert it.
- Regenerate and verify with `cargo xtask check-docs` (or `cargo check`) before handing work back.
- When changing generated API surface/docs for macro-backed types, update all four in the same PR: (1) macro source in `src/*.rs`, (2) generator template in `xtask/src/*_generated.rs`, (3) generated stub in `src/**/_generated.rs`, and (4) `xtask/src/main.rs` `check_generated_doc_stubs` expectations.

## Module Structure Convention

This project uses a specific module structure pattern. Do NOT create `mod.rs` files.

Correct pattern:

- `src/foo.rs` or `examples/foo.rs` (main module file)
- `src/foo/bar.rs` (submodule)
- `src/foo/baz.rs` (another submodule)

Incorrect pattern (never use):

- `src/foo/mod.rs` ❌
- `examples/foo/mod.rs` ❌

Example:

```rust
// File: src/led_strip.rs (main module)
pub mod esp32;

// File: src/led_strip/esp32.rs (submodule)
```

## Variable Naming Conventions

Variables should generally match their type names converted to snake_case. This improves predictability and encourages better type names.

Avoid abbreviations like `addrs`; spell out `addresses`.

### Naming: dimensions

Use standard Rust snake_case for locals, fields, and functions; UpperCamelCase for types; SCREAMING_SNAKE_CASE for constants.

Treat dimension markers like 12x4 and 3x4 as suffix qualifiers, not separate words.

Prefer `led12x4`, `font3x4`.

Avoid inserting an underscore before the dimension: avoid `led_12x4`, `font_3x4`.

For constants, keep underscores as word separators: prefer `LED_LAYOUT_12X4`, `FONT_4X6`, etc. (underscore before the dimension is fine in constants).

**Type-based naming:**

- `LedStrip` → `led_strip`
- `SosStrip` → `sos_strip`

**When to deviate:**

- Generic/contextual names are acceptable when the type is obvious and verbose naming would be redundant:
  - ✅ `spawner` (not `embassy_spawner`) - universally understood

**Single-character variables:**

Avoid single-character variables; use descriptive names:

- ❌ `i`, `j`, `x`, `y`, `a`, `b`
- ✅ `read_index`, `write_index`, `first_pixel`, `second_pixel`

**Project-specific patterns:**

- For the board peripherals handle from `esp_hal::init`, always use `init_and_start!(p)` so `p` is the consistent name across examples.

**Reference variables:**

When capturing variables in closures or creating references, append `_ref`:

- `led_strip` → `led_strip_ref`

## Comment Conventions

Use `TODO0`/`TODO00` prefix for TODO items (`TODO` + priority):

```rust
// TODO00 high priority task
// TODO0 lower priority consideration
// TODO lowest standard todo for general items
```

- For code that uses a stable workaround where a clearly better nightly feature exists, add:
  `// TODO_NIGHTLY When nightly feature <feature_name> becomes stable, change this code by <specific change>.`

Preserving comments: When changing code, generally don't remove TODO's in comments. Just move the comments if needed. If you think they no longer apply, add `(may no longer apply)` to the comment rather than deleting it.

- **Debug code policy**: Do not remove debug/test code, commented debugging blocks, or "THIS WORKS" / "THIS DOESN'T" comparison code until the bug is proven fixed. Leave diagnostic code in place even after identifying issues so the user can verify fixes work correctly before cleanup. This includes removing such comparisons when making edits—preserve them until explicit confirmation the fix is working.
- **Commit messages**: Always suggest a concise 1-2 line commit message when completing work (no bullet points, just 1-2 lines maximum). Present it in a fenced code block so it is easy to copy, for example:

  ```
  Fix SOS timing off-by-one in blinky
  ```

- **Publishing policy**: Agents must not run the real `cargo publish`. Prepare release notes/versioning/commands, but the actual publish step must be run by the person.
- Preserve comments: keep `TODO00`/`TODO0`/`TODO`, etc. comments. If they seem obsolete, append `(may no longer apply)` rather than deleting.

## Documentation Conventions

- Start module docs with "A device abstraction ..." and have them point readers to the main struct docs.
- Put a single compilable example on the primary struct; other public docs should link back to that example instead of duplicating snippets.
- When linking to module documentation, name the module in the link text (for example, "led_strip module documentation").
- When referring to examples, never say "struct-level example" or "module-level example". Use the name, for example: "LedStrip struct example" or "led_strip module example".

- **Markdown formatting**: When creating or editing markdown files, follow these rules to avoid linter warnings:
  - Add blank lines before and after lists (both bulleted and numbered)
  - Add blank lines before and after code blocks (fenced with triple backticks)
  - Add blank lines before and after headings
  - Ensure consistent list marker style within a file
  - Example violations to avoid:
    - `**Title:**` followed immediately by a list (needs blank line)
    - Code block followed immediately by text (needs blank line)
    - Heading followed immediately by another heading (needs blank line or text between)

When adding new examples, also add the standard cargo aliases (run + check) in `.cargo/config.toml` so they stay discoverable.

Use `cargo run --bin <name> --target riscv32imac-unknown-none-elf` as the standard way to run demos/examples; only use short alias commands when they are defined in `.cargo/config.toml`.

### Documentation Spec (for device modules)

- Module-level docs must start with "A device abstraction ..." and immediately direct readers to the primary public struct for details.
- Each module should have exactly one full, compilable example placed on the primary struct; keep other docs free of extra examples.
- Other public items (constructors, helper methods, type aliases) should point back to the primary struct's example rather than adding new snippets.
- **API completeness**: Every public method must either (1) have its own doc test, OR (2) be used in the struct's main example AND have a link from its doc comment pointing to that example (e.g., `See the [LedStrip struct example](Self) for usage.`). This ensures all functionality is documented and discoverable.
- **Duration clarity in public APIs**: For any public function/method that takes or returns a duration, write the type explicitly in the signature as `embassy_time::Duration`. In that function/method's doc comment, include a short sentence that explicitly states which duration type it uses.
- Examples should use the module's real constructors (e.g., `new_static`, `new`) and follow the device/static pair pattern shown elsewhere in the repo.
- Avoid unnecessary public type aliases; prefer private or newtype wrappers when exposing resources so internal types stay hidden.
- In examples, keep `use` statements limited to `device_envoy_esp32::...` items; refer to other crates/modules with fully qualified paths inline.
- Examples must show the actual `use` statements for the module being documented (bring types into scope explicitly rather than relying on hidden imports).

Spelling:

Use American over British spelling.

When making up variable names for examples and elsewhere, never use the prefix "My". Avoid this prefix.

- If an item comes from `crate`, `core`, `std`, or `alloc`, import it with `use` instead of using a fully-qualified `crate::`, `core::`, `std::`, or `alloc::` path in code. (Fully-qualified paths are fine in docs or comments.)
- Exception: for public function/method signatures with duration parameters/returns, use fully-qualified duration types per the Duration clarity policy above.
- In all demos, examples, and doctests, prefer condensed `use` statements (group related imports on a single `use` line where it stays readable).

Rust convention for getters/setters — no `get_` prefix for getters:

- Getters: `offset_minutes()`, `text()` (no prefix)
- Setters: `set_offset_minutes()`, `set_text()` (with `set_` prefix)

## Colors

For RGB8 colors, use the predefined constants from `device_envoy_esp32::led_strip::colors` rather than creating RGB values manually:

✅ Good:

```rust
use device_envoy_esp32::led_strip::colors;
let frame = Frame1d([colors::RED]);
```

❌ Bad:

```rust
use device_envoy_esp32::led_strip::RGB8;
let red = RGB8 { r: 255, g: 0, b: 0 };
```

Common colors available: `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`, etc.

## Terminology: "Panel" vs "Matrix"

Use **"NeoPixel-style (WS2812)"** for LED strip/pixel hardware. Always include the parenthetical "(WS2812)" to clarify the protocol.

Use **"panel"** when referring to physical rectangular LED display hardware composed of NeoPixel-style (WS2812) strips.

Use **"matrix"** for mathematical/algorithmic abstractions.

## LED Hardware Configuration

Pin assignments and portability guardrails are documented in `README.md` under `Default Pin Assignments`.
Treat that README section as the single source of truth for examples and docs.

## Device/Static Pair Pattern

Many drivers expose a `new_static` constructor for resources plus a `new` constructor for the runtime handle. We call this the **Device/Static Pair Pattern** and use it consistently across the repo.

- Always declare the static resources with `Type::new_static()` and name them `FOO_STATIC` when global.
- **Multi-instance devices** require passing `&TypeStatic` as the **first** argument when implementing or calling `Type::new`, named `<type>_static`.
- If `Spawner` is needed, place it as the **final** argument so everything else reads naturally between those bookends.
- **Static placement**: Place the static constructor on the line directly before the struct constructor. Don't group all statics at the top and all constructors below.

Example:

```rust
static SOS_STRIP_STATIC: SosStripStatic = SosStrip::new_static();
let sos_strip = SosStrip::new(&SOS_STRIP_STATIC, channel, spawner)?;
```

Don't ignore errors by assigning results to an ignored variable. Don't do this:

```rust
let _ = something_that_returns_a_result()
```

## API Design Patterns

**Avoid redundant API paths.** Prefer one clear way to do a thing unless there is a strong compatibility or interoperability reason.

- Do not expose both an associated const and an equivalent getter by default.
- If both are temporarily needed during migration, document the canonical one and plan to remove the duplicate.

**Avoid the builder pattern.** Users find builder patterns hard to discover. Instead:

- Use direct constructors with named parameters
- Take slices instead of requiring users to construct collections
- Return arrays/fixed-size types when possible rather than requiring users to build them

❌ Bad (builder pattern):

```rust
let display = DisplayBuilder::new()
    .width(12)
    .height(4)
    .build()?;
```

✅ Good (direct construction):

```rust
let display = Display::new(12, 4)?;
```

❌ Bad (forcing users to build collections):

```rust
let mut frames = Vec::new();
frames.push(frame1);
frames.push(frame2);
led.animate_frames(frames);
```

✅ Good (accept slices):

```rust
let frames = [frame1, frame2];
led.animate(&frames);
```

## Async Coordination

**Never use delays/timers to "fix" async coordination issues.** Delays like `Timer::after(Duration::from_millis(1))` to "let something finish" are evil — they're unreliable, hide the real problem, and make code fragile.

If async operations need coordination:

- Use proper synchronization primitives (Signals, Channels, Mutexes)
- Make operations synchronous if they don't need to be async
- Restructure the design to avoid the race condition
- Use acknowledgment/completion signals

❌ Bad (hoping a delay is long enough):

```rust
send_command().await;
Timer::after(Duration::from_millis(1)).await; // Evil!
let result = read_state();
```

✅ Good (proper coordination):

```rust
send_command().await;
wait_for_completion().await;
let result = read_state();
```

## Visibility and Documentation

When something shouldn't be in the public API docs, express that through visibility modifiers rather than doc attributes:

✅ Good:

```rust
pub(crate) struct InternalHelper { ... }  // Visible in crate, not in public docs
struct PrivateHelper { ... }              // Private, not in public docs
```

❌ Bad:

```rust
#[doc(hidden)]
pub struct InternalHelper { ... }  // Public but hidden - confusing!
```

If something truly shouldn't be in public docs, it shouldn't be `pub` either. Use `pub(crate)` for crate-internal APIs or omit `pub` entirely for private items.

### Exception: Macro helpers

There is one legitimate use case for `#[doc(hidden)]` on `pub` items: functions or re-exports called by public macros that expand at the call site. These must be `pub` (not `pub(crate)`) because macro-generated code in downstream crates needs to call them, but they're not part of the user-facing API.

```rust
#[doc(hidden)]
pub use esp_hal;  // Used by init_and_start! expansion in downstream crates
```

When using `#[doc(hidden)]` for this reason, always add a comment explaining why it must be public despite being an implementation detail.

For macro-helper functions, prefix helper names with `__` to clearly signal internal-only usage (for example, `__helper_for_macro`).
