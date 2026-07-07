<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# Spec: Fix "See the `Cyd` trait documentation for a usage example" claims

## Problem

In `crates/device-envoy-core/src/cyd.rs`, 14 method docs say:

> See the [`Cyd`] trait documentation for a usage example.

But the `Cyd` trait's single doctest only exercises three of those methods:

- `Cyd::parts` (via `Cyd::parts(&mut cyd)`)
- `CydDisplay::full_frame_mut`
- `CydTouch::read`

Every other method making the claim is **not** shown in that example, so the pointer is misleading. Readers land on the trait doc, search for the method name, and find nothing.

## Resolution strategy

Per repo convention (AGENTS.md): "For public methods, almost every method should either include its own doctest or link directly to a doctest that mentions the method."

Preferred fixes, in order:

1. **Add a doctest to the method's own doc** (best).
2. **Extend the trait doctest** to mention the method, only if it fits naturally without bloating the example.
3. **Drop the sentence** only for methods too trivial to warrant an example (avoid — use sparingly).

All new doctests follow existing conventions:

- Fence as `rust,no_run` is *not* required here — the existing trait doctest runs against `CydMemory` under the `host` feature; prefer runnable doctests gated the same way (`#[cfg_attr(feature = "host", doc = ...)]`) with assertions where cheap.
- Hide boilerplate (`CydMemory::new(...)`, imports, executor) with `#` lines.
- Use `futures_executor::block_on` only when the method is async-adjacent (frame flushing); pure getters need no executor.
- No `My`-prefixed names; American spelling.

## Method-by-method plan

### Trait `Cyd` (lines ~92–127)

| Method | Currently shown in trait doc? | Action |
| --- | --- | --- |
| `parts` | Yes | **Keep** the pointer as-is (claim is true). |
| `into_parts` | No | **Add method doctest**: build a `CydMemory`, call `into_parts()`, use each half, then `Cyd::from_parts` to reassemble. One doctest can be shared between `into_parts` and `from_parts` — put the full example on `into_parts`, and on `from_parts` say "See [`Cyd::into_parts`] for a round-trip example." |
| `from_parts` | No | Point to the `into_parts` doctest (above). |
| `display` | No | **Add small doctest**: `let display = cyd.display(); let _size = display.screen_size();` (host-gated, hidden setup). Alternatively extend the trait doctest — but the trait example deliberately uses `parts` to show simultaneous borrows, so a separate tiny doctest is cleaner. |
| `touch` | No | Same pattern as `display`: `cyd.touch().read()?`. |

### Trait `CydDisplay` (lines ~192–253)

| Method | Currently shown? | Action |
| --- | --- | --- |
| `screen_size` | No | **One shared doctest** covering the getter family: `screen_size`, `background`, `foreground`, `background_565`, `foreground_565`, `to_rgb565`. Place the full example on `screen_size`; each of the other five gets "See [`CydDisplay::screen_size`] for an example covering the device getter family." The example: construct `CydMemory`, assert `screen_size() == Size::new(320, 240)`, assert `background_565() == to_rgb565(background())`, same for foreground. |
| `background` | No | Point to `screen_size` shared doctest. |
| `foreground` | No | Point to `screen_size` shared doctest. |
| `background_565` | No | Point to `screen_size` shared doctest. |
| `foreground_565` | No | Point to `screen_size` shared doctest. |
| `to_rgb565` | No | Point to `screen_size` shared doctest. |
| `frame_mut_with_tile_top_left` | No | The tiled-drawing story presumably has an example on `CydDisplay::tiles` or in the display module. If so, change the pointer to that example ("See [`CydDisplay::tiles`] …"). If not, **add a method doctest** drawing into a tile at a nonzero `tile_top_left` and asserting a pixel via `CydMemory::pixel`. |
| `frame_mut` | No | **Add method doctest**: partial-rectangle frame — `frame_mut(Rectangle::new(Point::new(10, 10), Size::new(50, 40)))`, `fill`, `flush().await?`, assert a pixel inside and one outside the rectangle. |
| `full_frame_mut` | Yes | **Keep** the pointer (claim is true). |

### Trait `CydTouch`

| Method | Currently shown? | Action |
| --- | --- | --- |
| `read` | Yes | **Keep** the pointer (claim is true). |

`fill_rectangle` / `fill_contiguous` already point at `Self` ("related drawing APIs"), not at a usage example, so they are out of scope — though if effort is cheap, `fill_rectangle` would benefit from a doctest too (fill, assert pixel).

## Implementation notes

- All new doctests must be inside `#[cfg_attr(feature = "host", doc = r#"…"#)]` blocks like the existing trait example, because `CydMemory` is host-only.
- Reuse the existing trait doctest's setup shape (hidden `CydMemory::new(Size::new(320, 240), Rgb888::BLACK, Rgb888::WHITE, &FONT_9X15_BOLD)`) for consistency.
- When one doctest covers a family, each family member's doc must name the anchor method explicitly (repo rule: "make each method doc point to that example explicitly").
- The only other occurrence in submodules is `cyd/touch.rs:16` on `TouchEvent`; that claim is **true** (the trait doctest matches on `TouchEvent::Down`/`Move`), so leave it as-is.

## Verification

- `cargo test --doc -p device-envoy-core --features host` — all new doctests pass.
- `cargo doc -p device-envoy-core --features host` — visually confirm no remaining "See the [`Cyd`] trait documentation for a usage example" on a method the trait example does not demonstrate.
- `just check-all` before pushing.
