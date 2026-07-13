# Coding Notes for Agents

This file contains both shared workspace rules and crate-specific rules for this repository.

## General Policies

- In this repository, **devolve** means inlining a code element's behavior at
  its call sites (or into its containing function) and deleting the original
  struct, enum, helper, or other abstraction. Prefer the clearer term
  **inline** in prose when there is no need for the project-specific shorthand.

- **Never silently skip required build targets in xtask/CI.** Every supported target (e.g., ESP32-C6, ESP32-S3, Pico 1, Pico 2) must be built on every `check-all` run. If a required toolchain component is missing, fail loudly with a clear error message and instructions to install it — do not skip or silently ignore the missing target. Silent skips hide real breakage.
- When loading data from flash (or any other storage) into a local variable, name the variable after the concrete type. Example: `DeviceConfig` data should live in variables like `device_config`, not generic `config` or `flash0`.
- Avoid introducing `unsafe` blocks. If a change truly requires `unsafe`, call it out explicitly and explain the justification so the user can review it carefully.
- Avoid silent clamping; prefer asserts or typed ranges so out-of-range inputs fail fast.
- Prefer `no_run` doctests; use `ignore` only when absolutely necessary (and call out why). Running doctests is best when possible, but rarely feasible for embedded code.
- Always use `rust,no_run` in doctest fences, not just `no_run`.
- For programs that should run forever, use `pending().await` instead of a timer loop.
- **Hide boilerplate in doctests** using the `#` prefix (e.g., `# #![no_std]`). Hide lines that are noise to the reader but required for compilation: `#![no_std]`, `#![no_main]`, and standard imports like `use embassy_executor::Spawner;`. Keep only the essential code showing how to use the API. See the crate-specific sections below for platform-specific imports to hide or show.
- When adding docs for modules or public items, link readers to the primary struct and keep the single compilable example on that struct; other items should point back to it rather than duplicating examples.
- Prefer `const` values defined in the local context (inside the function/example) rather than at module scope when they're only used there.
- Do not add redundant `just` recipes that only mirror an existing `cargo` alias/command. If the behavior is the same, keep only the `cargo` command.
- For `cargo` aliases that target embedded triples, include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

## Dependency Upgrade Guardrails

- Treat embedded HAL/driver dependency upgrades as behavior migrations, not just compile fixes. When an upgraded API adds a required parameter, trace the previous behavior/default before choosing a placeholder value.
- Keep dependency-upgrade commits focused when possible. Avoid mixing broad docs/example churn with HAL, network, DMA, executor, or firmware-driver upgrades unless the extra changes are necessary for the migration.
- After upgrading hardware-facing dependencies, run at least one hardware smoke test for each affected subsystem before considering the upgrade complete. For RP WiFi/CYW43 changes, `clock_console` or a minimal WiFi example must get past CYW43 initialization and reach WiFi join or captive portal readiness.

## Generated Files

- Treat generated files under `src/**/_generated.rs` as build outputs, not source of truth.
- When changing generated docs/examples, edit the corresponding generator template in `xtask/src/*_generated.rs` first.
- If you must patch a generated file directly for an urgent fix, make the matching template change in the same PR so regeneration does not revert it.
- Regenerate and verify with `cargo xtask check-docs` (or the crate-level check command) before handing work back.
- When changing generated API surface/docs for macro-backed types, update all four in the same PR: (1) macro source in `src/*.rs`, (2) generator template in `xtask/src/*_generated.rs`, (3) generated stub in `src/**/_generated.rs`, and (4) `xtask/src/main.rs` `check_generated_doc_stubs` expectations.

## Module Structure Convention

This project uses a specific module structure pattern. Do NOT create `mod.rs` files.

- Macros related to a specific submodule should generally live in that submodule (for example, audio macros in `audio_player`) rather than in the top-level module.
- For exported `macro_rules!` macros that conceptually belong to a submodule, keep the user-facing docs/re-export in that submodule and avoid cluttering top-level macro docs. Prefer the existing pattern: `#[doc(hidden)]` on the `#[macro_export]` definition plus an in-module re-export (`pub use macro_name;`) with the full docs on that re-export.
- If you change macro visibility/export style, verify rustdoc placement still matches intent (submodule-focused docs, no unintended top-level macro listing).

Correct pattern:

- `src/foo.rs` or `examples/foo.rs` (main module file)
- `src/foo/bar.rs` (submodule)
- `src/foo/baz.rs` (another submodule)

Incorrect pattern (never use):

- `src/foo/mod.rs` ❌
- `examples/foo/mod.rs` ❌

Example:

```rust
// File: src/wifi_auto.rs (main module)
pub mod fields;
pub mod portal;

// File: src/wifi_auto/fields.rs (submodule)
// File: src/wifi_auto/portal.rs (submodule)
```

## Variable Naming Conventions

Variables should generally match their type names converted to snake_case. This improves predictability and encourages better type names.

Avoid abbreviations like `addrs`; spell out `addresses`.

### Naming: dimensions and 2d

Use standard Rust snake_case for locals, fields, and functions; UpperCamelCase for types; SCREAMING_SNAKE_CASE for constants.

Treat dimension markers like 12x4, 8x12, and 3x4 as suffix qualifiers, not separate words.

Prefer `led12x4`, `led8x12`, `font3x4`, `frame12x8_landscape`.

Avoid inserting an underscore before the dimension: avoid `led_12x4`, `font_3x4`.

Treat short semantic tags like 2d similarly: prefer `led2d`, avoid `led_2d`.

For constants, keep underscores as word separators: prefer `LED_LAYOUT_12X4`, `FONT_4X6`, etc. (underscore before the dimension is fine in constants).

**Type-based naming:**

- `WifiAuto` → `wifi_auto`
- `LedStrip` → `led_strip`

**When to deviate:**

- Generic/contextual names are acceptable when the type is obvious and verbose naming would be redundant:
  - ✅ `spawner` (not `embassy_spawner`) — universally understood

**Single-character variables:**

Avoid single-character variables; use descriptive names:

- ❌ `i`, `j`, `x`, `y`, `a`, `b`
- ✅ `read_index`, `write_index`, `first_pixel`, `second_pixel`

**Reference variables:**

When capturing variables in closures or creating references, append `_ref`:

- `led_strip` → `led_strip_ref`

## Comment Conventions

Use `TODO0*` for release-priority TODO items (`TODO` + one or more trailing `0`s):

```rust
// TODO00 high priority task
// TODO0 lower priority consideration
// TODO0000 release-blocking task with explicit emphasis
// TODO lowest standard todo for general items
```

- `TODO0*` (for example `TODO0`, `TODO00`, `TODO0000`) means action is required before the next release.
- Plain `TODO` means later/non-release work unless explicitly stated otherwise.
- For code that uses a stable workaround where a clearly better nightly feature exists, add:
  `// TODO_NIGHTLY When nightly feature <feature_name> becomes stable, change this code by <specific change>.`
- Preserving comments: When changing code, generally don't remove TODO's in comments. Just move the comments if needed. If you think they no longer apply, add `(may no longer apply)` to the comment rather than deleting it.
- **Debug code policy**: Do not remove debug/test code, commented debugging blocks, or "THIS WORKS" / "THIS DOESN'T" comparison code until the bug is proven fixed. Leave diagnostic code in place even after identifying issues so the user can verify fixes work correctly before cleanup. This includes removing such comparisons when making edits—preserve them until explicit confirmation the fix is working.
- **Commit messages**: Always suggest a concise 1-2 line commit message when completing work (no bullet points, just 1-2 lines maximum). Present it in a fenced code block so it is easy to copy.
- **Publishing policy**: Agents must not run the real `cargo publish`. Prepare release notes/versioning/commands, but the actual publish step must be run by the person.

## Documentation Conventions

- Start module docs with "A device abstraction ..." and have them point readers to the main struct docs.
- Put a single compilable example on the primary struct; other public docs should link back to that example instead of duplicating snippets.
- When linking to module documentation, name the module in the link text (for example, "led_strip module documentation").
- When linking to items in rustdoc, prefer readable link text such as `MemoryCyd` with an explicit target (`[MemoryCyd](crate::memory::MemoryCyd)`) instead of exposing raw `crate::...` paths in the rendered text.
- When referring to examples, never say "struct-level example" or "module-level example". Use the name, for example: "WifiAuto struct example" or "led_strip module example".

**Markdown formatting**: When creating or editing markdown files, follow these rules to avoid linter warnings:

- Add blank lines before and after lists (both bulleted and numbered)
- Add blank lines before and after code blocks (fenced with triple backticks)
- Add blank lines before and after headings
- Ensure consistent list marker style within a file
- Example violations to avoid:
  - `**Title:**` followed immediately by a list (needs blank line)
  - Code block followed immediately by text (needs blank line)
  - Heading followed immediately by another heading (needs blank line or text between)

When adding new examples, also add the standard cargo aliases in `.cargo/config.toml` so they stay discoverable.

### Documentation Spec (for device modules)

- Module-level docs must start with "A device abstraction ..." and immediately direct readers to the primary public struct for details.
- Each module should have exactly one full, compilable example placed on the primary struct; keep other docs free of extra examples.
- Other public items (constructors, helper methods, type aliases) should point back to the primary struct's example rather than adding new snippets.
- **API completeness**: Every public method must either (1) have its own doc test, OR (2) be used in the struct's main example AND have a link from its doc comment pointing to that example. This ensures all functionality is documented and discoverable.
- **Duration clarity in public APIs**: For any public function/method that takes or returns a duration, write the type explicitly in the signature (do not use bare `Duration`). In that function/method's doc comment, include a short sentence that explicitly states which duration type it uses.
- Examples should use the module's real constructors (e.g., `new_static`, `new`) and follow the device/static pair pattern shown elsewhere in the repo.
- Avoid unnecessary public type aliases; prefer private or newtype wrappers when exposing resources so internal types stay hidden.
- Examples must show the actual `use` statements for the module being documented (bring types into scope explicitly rather than relying on hidden imports).
- In all demos, examples, and doctests, prefer condensed `use` statements (group related imports on a single `use` line where it stays readable).

### Core-Only Trait Example Workflow

- When creating core-only trait examples for a device abstraction, first extract the canonical struct/module examples from `device-envoy-rp` into `device-envoy-examples-esp/examples` as `*_exampleN_trait.rs` files.
- Treat these ESP trait examples as an editable staging area and let the user iterate on them before changing docs.
- Only after explicit user approval, move the finalized examples into trait documentation and reuse the introductory wording from the RP struct docs where applicable.
- Before writing or editing any `*_trait.rs` example, review all existing `*_trait.rs` examples in the crate to match the established style and structure.
- For `*_trait.rs` examples, include a core-only example function that uses core-only imports for the trait-facing API; follow the `audio_exampleNUM_trait.rs` pattern.
- When migrating staged examples into a trait's documentation, add `See the FILLIN trait documentation for usage examples.` (with a proper link) on every method or constant doc that is demonstrated by those examples.
- In trait-migration examples and doctests, prefer inline trait bounds (`impl Trait<...>`) over `where` clauses when readability allows.
- In trait-migration examples and doctests, avoid placeholder generic type names like `WifiAutoType`; use `impl Trait` or a concrete descriptive type name when a type parameter is required.

### CYD Doctest Previews

- Every CYD-related doctest that renders visible output should have a generated golden PNG and assert against it, typically in hidden doctest lines so the visible example stays focused on API usage.
- Every CYD-related doctest that renders visible output should also show a preview image in the rendered docs, and that preview should appear after the code example, not before it.
- For rustdoc-hosted CYD previews, prefer the existing `embed-doc-image` pattern with assets checked into `docs/assets`; place the embedded-image attribute on the same documented item that renders the preview.

Spelling: use American over British spelling.

When making up variable names for examples and elsewhere, never use the prefix "My". Avoid this prefix.

- If an item comes from `crate`, `core`, `std`, or `alloc`, import it with `use` instead of using a fully-qualified `crate::`, `core::`, `std::`, or `alloc::` path in code. (Fully-qualified paths are fine in docs or comments.)
- Exception: for public function/method signatures with duration parameters/returns, use fully-qualified duration types per the Duration clarity policy above.

Rust convention for getters/setters — no `get_` prefix for getters:

- Getters: `offset_minutes()`, `text()` (no prefix)
- Setters: `set_offset_minutes()`, `set_text()` (with `set_` prefix)

### Parsing into a Stronger Type

Prefer shadowing when converting from weaker to stronger types (e.g., parsing strings):

```rust
let width = width.parse::<u32>()?;
```

Guidelines:

- Prefer shadowing at the smallest reasonable scope so the "new" meaning doesn't leak too far.
- Use assertions or checked conversions before shadowing when truncation/overflow is possible.
- Don't shadow across long spans if it could confuse readers—shadow near the point of use.

## Terminology: "Panel" vs "Matrix"

Use **"NeoPixel-style (WS2812)"** for LED strip/pixel hardware. Always include the parenthetical "(WS2812)" to clarify the protocol, not just "WS2812-style" or bare "WS2812".

Use **"panel"** when referring to physical rectangular LED display hardware composed of NeoPixel-style (WS2812) strips:

- ✅ "LED panel" — A physical rectangular arrangement of LED strips (e.g., 12×4 pixels)
- ✅ "Multiple panels" — Several rectangular units combined or stacked
- ✅ Used in: hardware setup documentation, example titles, user-facing descriptions

Use **"matrix"** for mathematical/algorithmic abstractions:

- ✅ `BitMatrix` — Internal data structure representing segment patterns
- ✅ `led2d` module — Refers to 2D array abstraction
- ✅ Used in: type names, internal algorithms, mathematical contexts

This distinction clarifies that panels are physical hardware while matrices are logical data structures.

## Device/Static Pair Pattern

Many drivers expose a `new_static` constructor for resources plus a `new` constructor for the runtime handle. We call this the **Device/Static Pair Pattern** and use it consistently across the repo.

- Always declare the static resources with `Type::new_static()` and name them `FOO_STATIC` when global.
- If `Spawner` is needed, place it as the **final** argument so everything else reads naturally between those bookends.
- **Static placement**: Place the static constructor on the line directly before the struct constructor. Don't group all statics at the top and all constructors below.

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

## Trait-Only Migration Pattern (Led2d)

When migrating a device abstraction from generated inherent methods/consts to a pure trait API (constructors excluded), follow this sequence.

Core rule: identify the smallest primitive method set first, then define all non-primitive behavior as default trait methods expressed in terms of those primitives and associated consts.
Naming rule for migrations: keep canonical abstraction names on traits (`IrKepler`, etc.) and rename platform concrete structs with explicit suffixes (`IrKeplerRp`, `IrKeplerEsp`, etc.) to avoid collisions.
Core boundary rule: `device-envoy-core` should define the canonical trait and shared value types; avoid adding platform runtime handle structs there unless they are truly platform-agnostic and intended as canonical API surface.
Platform runtime rule: if macro/device plumbing needs a runtime handle struct, define it in the platform crate (`device-envoy-rp` / `device-envoy-esp`) and keep it as implementation detail, not the canonical API.
Runtime naming rule: when such platform runtime structs are needed, name them using the abstraction name + platform suffix (`XRp`, `XEsp`), for example `LedStripRp` / `LedStripEsp`.

1. If no trait exists yet, introduce one first (in `device-envoy-core`) with the target API shape.
2. Keep the old inherent surface temporarily while wiring the new trait, so callsites can migrate incrementally.
3. Define the canonical trait in `device-envoy-core`.
4. Move the API surface to trait items:
   - Required associated consts for implementation-specific values (for Led2d: `MAX_FRAMES`, `MAX_BRIGHTNESS`, `FONT`)
   - Derived/default associated consts from trait const generics when possible (for Led2d: `WIDTH`, `HEIGHT`, `LEN`, geometry points/sizes)
   - Primitive required methods (for Led2d: `write_frame`, `animate`)
   - Default helper methods built on primitives + associated consts (for Led2d: `write_text_to_frame`, `write_text`)
5. Keep constructors inherent (`new`, `new_static`, `from_*`) and out of the trait unless there is a strong reason otherwise.
6. In platform macro expansions, implement the core trait for each generated type and provide all required consts/methods there.
7. Keep platform runtime/plumbing types out of the abstraction docs and out of "Start Here" guidance; docs should point readers to the canonical trait and trait methods.
8. Remove duplicated inherent API from generated types:
   - Remove inherent API consts that now live on the trait
   - Remove inherent helper methods that are now trait defaults
   - Remove now-unneeded stored fields used only by removed inherent helpers
9. Enforce trait-based method resolution for non-constructor APIs:
   - After migration, methods like `write_*`, `animate`, `play`, etc. must resolve via the trait, not inherent methods.
   - Do not add inherent forwarding/wrapper methods (including UFCS shims) for trait operations during migration.
   - Do not expose those methods through `Deref` to runtime helper structs.
   - Keep constructors (`new`, `new_static`, `from_*`) inherent; move operational methods to trait-only surface.
   - Verify at least one representative callsite requires `use ...::<TraitName> as _;` for method resolution.
10. Update callsites to use trait methods/consts:
   - Bring the trait into scope as `_` for method resolution (`use ...::Led2d as _;`)
   - For associated const access, use UFCS (`<Type as Led2d<W, H>>::CONST` or equivalent reference type form)
11. Move docs to the trait as the API reference:
   - Update module "Start Here" links to point at the trait and trait methods
   - Replace references to generated sample types with trait references in macro docs and module docs
   - For macro-backed abstractions, keep a representative generated doc page when it improves constructor discoverability (`new`, `new_static`) or sample-type navigation in rustdoc.
12. Keep associated constants visible in both places when useful:
   - Keep canonical semantics on the trait (for generic code and UFCS access).
   - Also expose matching inherent constants on generated types when users commonly read them before construction or from generated-type pages.
   - Keep names and values aligned between trait and generated type; avoid one-off aliases.
13. Remove generated doc stub flow when it no longer represents the API:
   - Delete the abstraction's `*_generated.rs` generator/template and generated stub module
   - Remove `xtask` generation/check hooks for that stub, including `check_generated_doc_stubs` expectations
   - Update crate `AGENTS.md` generated-file lists accordingly
14. Keep compatibility aliases only when needed for migration, and document canonical names.
15. Verify with crate/workspace checks (`cargo xtask check-all` and docs checks) so trait-const access and imports are validated across examples/demos/tests.

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

When using `#[doc(hidden)]` for this reason, always add a comment explaining why it must be public despite being an implementation detail.

For macro-helper functions, prefix helper names with `__` to clearly signal internal-only usage (for example, `__helper_for_macro`).

## Tips for Unifying Code

These tips apply when moving platform-specific code into `device-envoy-core` or otherwise consolidating shared logic across crates.

- **Port full abstraction families, not only top-level files.** When asked to port a device abstraction `X` between platform crates, port all submodules and macro/helper pieces that make up the user-facing abstraction (for example `button` plus `button_watch`), then update examples/docs to use the ported abstraction rather than one-off local replacements.

- **Inline trivial re-export modules.** When a submodule file is reduced to just a
  `pub use some_crate::some_module::*;` re-export (a few lines), don't keep it as a
  standalone file. Instead, inline it as a one-liner `pub mod` block directly in the
  parent module:

  ```rust
  // In led2d.rs — no separate layout.rs file needed
  pub mod layout {
      pub use device_envoy_core::led2d::layout::*;
  }
  ```

  Delete the now-empty subdirectory too. This keeps the file tree clean and avoids
  file proliferation for files with essentially no content.

- **Rename hardware-named files to abstraction-named files.** Platform crates (e.g.,
  `device-envoy-esp`) used to mix platform-independent code and ESP-specific
  implementation code in the same files, with the implementation details sometimes
  living in hardware-named files like `esp32.rs`. As the platform-independent code
  is extracted into `device-envoy-core`, what remains in the platform crate is
  purely the ESP-specific wiring. At that point, rename the file to match the device
  abstraction it implements — e.g., the ESP-specific parts of `led_strip` belong in
  `led_strip.rs`, not in a file named after the chip (`esp32.rs`). File names should
  describe the abstraction, not the underlying hardware.

## Crate-Specific Policies (device-envoy-rp / Pico)

- While the crate version remains `0.0.3-alpha`, we do not care about breaking changes. Optimize for the best API design.
- For Pico programs that should run forever, use `pending().await` instead of a timer loop.
- **Hide boilerplate in doctests**: In addition to shared rules, hide `use panic_probe as _` and `use defmt_rtt as _`. **Important:** Do NOT hide imports from `device_envoy_rp`, `embassy_time::Duration`, `smart_leds`, or `embedded_graphics` because they are unusual and users need to see them to understand what to import.
- Always run `cargo check-all` before handing work back; xtask keeps doctests and examples in sync.
- Do not add redundant `just` recipes that only mirror an existing `cargo` alias/command. If the behavior is the same, keep only the `cargo` command.
- For `cargo` aliases that target embedded triples (`thumbv6m-none-eabi`, `thumbv8m.main-none-eabihf`, or `riscv32imac-unknown-none-elf`), include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

### Generated Files (RP)

For this crate, generation is wired through `xtask` for: `audio_player_generated`, `audio_clip_generated`, `ir_generated`, `lcd_text_generated`, `led_generated`, `led_strip_generated`, and `servo_player_generated`.

### Const-Only APIs (RP)

**The `LedLayout` type must remain fully const.** All methods on `LedLayout` must be `const fn`. This enables compile-time LED layout validation and zero-runtime-cost transformations. If you add a method to `LedLayout` that is not `const fn`, report this as an error. The existing doctests enforce const-ness by using methods in const contexts; removing `const` from any method will cause compilation to fail.

### Variable Naming Conventions (RP-specific)

**Type-based naming:**

- `Led12x4` -> `led12x4` (dimension suffix)
- `WifiAuto` -> `wifi_auto`
- `LedStrip` -> `led_strip`
- `Led12x4ClockDisplay` -> `led12x4_clock_display`

**When to deviate:**

- Generic/contextual names are acceptable when the type is obvious and verbose naming would be redundant:
  - `button` (not `button_pico2`) when only one button exists
  - `clock` (not `clock_0`) when context is clear

**Project-specific patterns:**

- For the board peripherals handle from `embassy_rp::init`, always use the shorthand `let p = embassy_rp::init(...)` so examples stay consistent.

**Reference variables:**

- `led12x4` -> `led12x4_ref`
- `wifi_auto` -> `wifi_auto_ref`

### Terminology (RP-specific)

- **PIO resource** (not "PIO block") — Use "PIO resource" or just "PIO" when referring to the PIO peripheral.

### PIO IRQ Mapping (RP-specific)

- Use the shared `crate::pio_irqs::PioIrqMap` trait when a module only needs to map a PIO resource to `Irqs` + `irqs()`.
- If a module needs extra PIO-specific behavior (for example task spawning hooks), define a module-local trait that extends `PioIrqMap` and only add the extra methods there.

### Colors (RP-specific)

For RGB8 colors, use the predefined constants from `smart_leds::colors` (re-exported from `led_strip::colors`) rather than creating RGB values manually:

Good:

```rust
use device_envoy_rp::led_strip::colors;
let frame = [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW];
```

Bad:

```rust
use device_envoy_rp::led_strip::Rgb;
let red = Rgb::new(255, 0, 0);
let green = Rgb::new(0, 255, 0);
```

Common colors available: `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`, etc.

When working directly with the `embedded_graphics` crate, using `colors::RED.to_rgb888()` (with `device_envoy_rp::led_strip::ToRgb888` in scope) is acceptable to avoid conversions.

### Device/Static Pair Pattern (RP-specific)

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

### Documentation (RP-specific)

- In examples, keep `use` statements limited to `device_envoy_rp::...` items; refer to other crates/modules with fully qualified paths inline.
- Keep example shape consistent: show an async function that receives `Peripherals`/`Spawner` (or other handles) and constructs the device with `new_static`/`new`; avoid mixing inline examples without that pattern next to function-based ones.
- In examples, prefer importing the types you need (`use crate::foo::{Device, DeviceStatic};`) instead of fully-qualified paths for statics.
- Use `cargo run --bin <name> --target <target> --features <features>` as the standard way to run demos/examples; only use short `cargo demo-*` commands when they are defined as aliases in `.cargo/config.toml`.
- **API completeness**: When linking back to the primary struct example, use phrasing like `See the [WifiAuto struct example](Self) for usage.`

#### Style Macro Documentation (RP-specific)

For style macros (for example `audio_clip!`, `audio_player!`, `led_strip!`), document them with a consistent structure:

1. One-line summary
2. Compact syntax block
3. Inputs (`$vis`, `$name`, etc.) including which are optional
4. Required fields
5. Optional fields/defaults
6. Link to the module documentation for full usage examples

Whenever a style macro implementation or its docs change, verify that macro docs and behavior stay in sync (accepted fields, defaults, optional inputs, generated items, and linked examples).

### LED Hardware Configuration (RP-specific)

Examples use the following standard PIO resource and pin assignments:

- **PIO resource 0 + PIN_0** — 8 LEDs in a line (e.g., `led_strip_single.rs`)
- **PIN_3** — 12x4 panel (48 pixels, e.g., `led_strip_3_on_a_pio.rs`)
- **PIN_4** — Two 12x4 panels combined into 12x8 panel (96 pixels)

When writing new examples or documentation, follow this convention for consistency.

#### Single LED Wiring (RP-specific, `Led` device)

For single LED examples using the `Led` device abstraction, use **PIN_1**. The `Led` device supports both high-level-on and low-level-on configurations:

**High level on (default):**

- LED anode (long leg) -> 220ohm resistor -> PIN_1
- LED cathode (short leg) -> GND
- Use: `Led::new(&led_static, pin, OnLevel::High, spawner)`

**Low level on:**

- LED anode (long leg) -> 3.3V
- LED cathode (short leg) -> 220ohm resistor -> PIN_1
- Use: `Led::new(&led_static, pin, OnLevel::Low, spawner)`

The `OnLevel` enum specifies what pin level turns the LED on.

#### Button Pin (RP-specific)

The standard button pin across examples is **PIN_13**:

```rust
let mut button = ButtonRp::new(p.PIN_13, PressedTo::Ground);
```

Use this consistently when adding button input to examples.

#### Servo Pins (RP-specific)

The standard servo pins across examples are **PIN_11** and **PIN_12**.

## Crate-Specific Policies (device-envoy-esp / ESP32)

- While the crate version remains `0.0.4-alpha.2`, we do not care about breaking changes. Optimize for the best API design.
- For ESP32 programs that should run forever, use `pending().await` instead of a timer loop.
- **Hide boilerplate in doctests**: In addition to shared rules, hide `use esp_backtrace as _`. **Important:** Do NOT hide imports from `device_envoy_esp`, `embassy_time::Duration`, or `smart_leds` because they are unusual and users need to see them to understand what to import.
- Always run `cargo check` before handing work back.
- For `cargo` aliases that target `riscv32imac-unknown-none-elf`, include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

### Generated Files (ESP)

For this crate, generation is wired through `xtask` for: `audio_player_generated`, `audio_clip_generated`, `ir_generated`, and `led_generated`.

- Board examples under `crates/device-envoy-examples-esp/examples/<chip>/<board>/` are generated outputs. Add or change them by editing `crates/device-envoy-examples-esp/examples/templates/*.rs.j2` (or `crates/device-envoy-examples-esp/examples/templates/talk1/*.rs.j2`) and then running `cargo run --manifest-path crates/device-envoy-examples-esp/xtask/Cargo.toml -- generate-board-examples`.
- Do not hand-edit generated board example files as source-of-truth; template changes should drive regeneration.
- In `crates/device-envoy-examples-esp/Cargo.toml`, do not manually edit generated example entries. The block between `# BEGIN GENERATED BOARD EXAMPLES` and `# END GENERATED BOARD EXAMPLES` is generated by xtask.
- For board-template examples, prefer board-rendered template variables over chip-family `#[cfg(...)]` branching for pin selection so generated files are concrete per board.
- Generated board example files should start with:
  `// @generated <template-path> by cargo xtask generate-board-examples.`
  and may include a second line with regeneration guidance.
- After changing board example templates or board example generation logic, run:
  `cargo xtask generate-board-examples`
  followed by
  `cargo check --manifest-path crates/device-envoy-esp/Cargo.toml`.

### main/inner_main Pattern (ESP)

Use the `main`/`inner_main` split to allow the `?` operator in example and demo code:

```rust
use core::{convert::Infallible, future::pending};

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let err = inner_main(spawner).await.unwrap_err();
    panic!("{err:?}");
}

async fn inner_main(spawner: Spawner) -> device_envoy_esp::Result<Infallible> {
    init_and_start!(p);
    // ... use ? freely here ...
    pending().await
}
```

This pattern keeps `main` free of `?` (which `-> !` forbids) while keeping `inner_main` ergonomic.
Prefer consuming the `Result<Infallible>` post-condition with `unwrap_err()` in `main` rather than re-matching the unreachable `Ok` branch.

**File layout:** order top-level items as includes/constants, then `main`/`inner_main` (the entry point), then error types, other helper types, and helper functions. Error enums (e.g. `MainError`) and their `From` impls belong after `inner_main`, not before it — readers should see the program flow before its error plumbing.

### init_and_start! Macro (ESP)

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

### Variable Naming Conventions (ESP-specific)

**Type-based naming:**

- `LedStrip` -> `led_strip`
- `SosStrip` -> `sos_strip`

### Colors (ESP-specific)

For RGB8 colors, use the predefined constants from `device_envoy_esp::led_strip::colors` rather than creating RGB values manually:

Good:

```rust
use device_envoy_esp::led_strip::colors;
let frame = Frame1d([colors::RED]);
```

Bad:

```rust
use device_envoy_esp::led_strip::RGB8;
let red = RGB8 { r: 255, g: 0, b: 0 };
```

Common colors available: `RED`, `GREEN`, `BLUE`, `YELLOW`, `WHITE`, `BLACK`, `CYAN`, `MAGENTA`, `ORANGE`, `PURPLE`, etc.

### Device/Static Pair Pattern (ESP-specific)

**Multi-instance devices** require passing `&TypeStatic` as the **first** argument when implementing or calling `Type::new`, named `<type>_static`.

Example:

```rust
static SOS_STRIP_STATIC: SosStripStatic = SosStrip::new_static();
let sos_strip = SosStrip::new(&SOS_STRIP_STATIC, channel, spawner)?;
```

### Porting Scope (ESP-specific)

- When porting a device abstraction from another platform crate, port all user-facing sibling submodules/helpers that belong to that abstraction (for example `button` and `button_watch`) instead of adding example-local replacements.
- For LEDC timer/channel resources, follow the crate ownership-claim protocol used by servo abstractions. Do not bypass this protocol with direct ad-hoc LEDC timer/channel setup in examples or device modules.
- If a macro/device abstraction claims an LEDC timer or channel, treat that claim as exclusive for the entire binary (no sharing), and let duplicate usage fail at link time.

### LED Hardware Configuration (ESP-specific)

Use the ESP capability/board-profile system to choose GPIOs; do not assume one fixed pin map across chips.

- For board-generated examples, treat `crates/device-envoy-examples-esp/xtask/src/boards.rs` (`BOARD_PROFILES`) as source of truth.
- For runtime chip/feature behavior, rely on the capability system (`Capability`, `EspCurrentCapabilities`, and capability checks) rather than hardcoded chip-name pin assumptions.
- When writing new examples or docs, describe pins as board/chip dependent and point to board profiles/capability-driven configuration.

#### Button Pin (ESP-specific)

Button GPIO is board-profile/capability driven (do not assume one universal pin).

```rust
let mut button = ButtonEsp::new(p.GPIO6, PressedTo::Ground);
```

If you use a concrete pin in an example, make sure it matches the selected board profile.

#### Servo Pins (ESP-specific)

Servo GPIO selection is board-profile/capability driven (do not assume one universal pair).

#### LCD Text I2C Pins (ESP-specific)

LCD text I2C pin selection is board-profile/capability driven (SDA/SCL are board dependent).

#### Audio Player I2S Pins (ESP-specific)

Audio player I2S pin selection is board-profile/capability driven (`DIN`/`BCLK`/`LRC` are board dependent).

### Documentation (ESP-specific)

- In examples, keep `use` statements limited to `device_envoy_esp::...` items; refer to other crates/modules with fully qualified paths inline.
- Use `cargo run --bin <name> --target riscv32imac-unknown-none-elf` as the standard way to run demos/examples; only use short alias commands when they are defined in `.cargo/config.toml`.
- **API completeness**: When linking back to the primary struct example, use phrasing like `See the [LedStrip struct example](Self) for usage.`

### Visibility and Documentation (ESP-specific)

One common `#[doc(hidden)]` + `pub` case in this crate:

```rust
#[doc(hidden)]
pub use esp_hal;  // Used by init_and_start! expansion in downstream crates
```
