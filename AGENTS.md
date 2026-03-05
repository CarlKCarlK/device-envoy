# Coding Notes for Agents

Crate-specific rules are in each crate's `AGENTS.md`. This file contains rules that apply to all crates in this workspace.

## General Policies

- **Never silently skip required build targets in xtask/CI.** Every supported target (e.g., ESP32-C6, ESP32-S3, Pico 1, Pico 2) must be built on every `check-all` run. If a required toolchain component is missing, fail loudly with a clear error message and instructions to install it — do not skip or silently ignore the missing target. Silent skips hide real breakage.
- When loading data from flash (or any other storage) into a local variable, name the variable after the concrete type. Example: `DeviceConfig` data should live in variables like `device_config`, not generic `config` or `flash0`.
- Avoid introducing `unsafe` blocks. If a change truly requires `unsafe`, call it out explicitly and explain the justification so the user can review it carefully.
- Avoid silent clamping; prefer asserts or typed ranges so out-of-range inputs fail fast.
- Prefer `no_run` doctests; use `ignore` only when absolutely necessary (and call out why). Running doctests is best when possible, but rarely feasible for embedded code.
- Always use `rust,no_run` in doctest fences, not just `no_run`.
- For programs that should run forever, use `core::future::pending().await` instead of a timer loop.
- **Hide boilerplate in doctests** using the `#` prefix (e.g., `# #![no_std]`). Hide lines that are noise to the reader but required for compilation: `#![no_std]`, `#![no_main]`, and standard imports like `use embassy_executor::Spawner;`. Keep only the essential code showing how to use the API. See the crate-level `AGENTS.md` for which platform-specific imports to hide or show.
- When adding docs for modules or public items, link readers to the primary struct and keep the single compilable example on that struct; other items should point back to it rather than duplicating examples.
- Prefer `const` values defined in the local context (inside the function/example) rather than at module scope when they're only used there.
- Do not add redundant `just` recipes that only mirror an existing `cargo` alias/command. If the behavior is the same, keep only the `cargo` command.
- For `cargo` aliases that target embedded triples, include `--no-default-features` unless there is an explicit, documented reason to keep default features enabled.

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

Use `TODO0`/`TODO00` prefix for TODO items (`TODO` + priority):

```rust
// TODO00 high priority task
// TODO0 lower priority consideration
// TODO lowest standard todo for general items
```

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
9. Update callsites to use trait methods/consts:
   - Bring the trait into scope as `_` for method resolution (`use ...::Led2d as _;`)
   - For associated const access, use UFCS (`<Type as Led2d<W, H>>::CONST` or equivalent reference type form)
10. Move docs to the trait as the API reference:
   - Update module "Start Here" links to point at the trait and trait methods
   - Replace references to generated sample types with trait references in macro docs and module docs
11. Remove generated doc stub flow when it no longer represents the API:
   - Delete the abstraction's `*_generated.rs` generator/template and generated stub module
   - Remove `xtask` generation/check hooks for that stub, including `check_generated_doc_stubs` expectations
   - Update crate `AGENTS.md` generated-file lists accordingly
12. Keep compatibility aliases only when needed for migration, and document canonical names.
13. Verify with crate/workspace checks (`cargo xtask check-all` and docs checks) so trait-const access and imports are validated across examples/demos/tests.

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
