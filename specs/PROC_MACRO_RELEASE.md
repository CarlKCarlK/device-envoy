# Proc-Macro Release Plan

Tracks the work between the `proc-macro-const-structures` branch and a
published Device Envoy release whose declaration macros are `macro-schema`
schemas. Tick an item in the same commit that completes it. Delete this file
once the release has shipped.

## 1. Port the macros — done

All 33 declaration macros are `define!` schemas with `generate` templates. The
inventory, the API regularization, and the macros intentionally left as
`macro_rules!` are recorded in
[`macro-schema/specs/DE_MACRO_SURVEY.md`](https://github.com/CarlKCarlK/macro-schema/blob/main/specs/DE_MACRO_SURVEY.md).

## 2. `macro-schema` dependency

- [x] Publish `macro-schema` 0.1.0 and depend on it from crates.io (`3d190d09`).
- [x] Publish `macro-schema` 0.1.1 with `macro_schema_core::expand_definition`
      and depend on it (2026-10-22, `macro-schema` `18565c7`).
- Republishing is fine: no one else uses these crates. Any framework change
  this release needs goes in a new `macro-schema` version, published before
  Device Envoy.

## 2a. Code review of the macro code

- [ ] Review the organization of the `define!` schemas and `generate`
      templates across Core, RP, and ESP: where each lives, how the
      single-as-group and group-calls-single patterns are applied, and the
      remaining hand-written helpers. All families reviewed 2026-11-05.
      Findings:
  - [x] Bug: on ESP, a second `button_watch!` button failed at runtime with
        `Error::TaskSpawn`. Every button spawned one crate-level task with the
        default `pool_size` of 1. `button_watch_five_compile` only compiles,
        so it never caught this. Fixed 2026-11-05: each declaration generates
        its own task, as on RP.
  - [x] The button-watch runtime (static, handle, and press loop, about 180
        lines) was duplicated in RP and ESP, identical apart from an import
        path. It moved to core (`button/watch.rs`, 2026-11-05). Each chip
        keeps only its HAL-specific `InputButton` and a hidden
        `__button_watch_task`. The task now takes the whole
        `&'static ButtonWatchStatic` instead of six separate references, and
        the never-read `state_changed_signal` and unused
        `wait_for_state_change` were deleted.
  - [x] Fixed 2026-11-05 (`9df65cad`): generated `new` docs drift. RP `servo_player!` is wrong: it says the
        slice "is automatically determined from the pin via the type system",
        gives a stale slice formula, and points to a nonexistent "`ServoPlayer`
        struct example". ESP `servo_player!` is a bare one-liner. The verbs
        vary ("Create", "Construct", "Creates and spawns the generated …
        instance"). Use "Creates the … and spawns its background task", and
        give `servo_player!` the same `slice` text as `servo!`.
  - The panel wrapper (about 120 lines: inherent consts, corners, `strip()`,
    and the `Led2d` impl) is the same template text in RP `led_strips!` and
    ESP `led2d!`. That is expected under the 2026-10-23 decision to keep each
    chip's templates separate. Moving the inherent items into provided trait
    items would cost users their import-free `Panel::WIDTH`, so it stays.
  - LED, audio, servo, and LCD: no within-chip duplication worth removing.
    The templates delegate to shared runtime (`servo_from_pin_slice`, core
    audio and I2C loops).
  - The organization is sound. Each family is one file, with its schemas
    next to the runtime code they call. The pattern follows the hardware:
    RP uses single-as-group because members share a PIO block (`ir!`,
    `ir_kepler!`, `ir_mapping!`, `led_strip!`, `lcd_text!` are one-member
    groups). ESP uses group-calls-single because each IR receiver owns its
    own RMT channel.
  - The remaining `macro_rules!` are justified: `combine!`, `tone!`,
    `init_and_start!`, and table generators that stamp out one item per
    peripheral (`__define_ir_task!`, `impl_wifi_pio!`, `servo_pin_map!`,
    `impl_dma_irq_map_all_irqs!`, `__cyd_tga!`,
    `__impl_wifi_auto_fields!`). `__led_engine_normalize!` no longer exists.
  - The repeated field doc "Each PIO serves one device or group…" (10 copies
    in 4 RP files) stays inline (decided 2026-11-05). Users must see it on
    each macro page without following a link, and the cost of repeating it
    falls only on maintainers. If it ever needs editing, change all copies
    in one commit.
  - Skipped for this release (decided 2026-11-05): fold Kepler into mapping.
        The RP-only version would reintroduce an API difference between the
        chips, and the version for both chips adds a hidden schema layer to
        save about 30 lines. Revisit after the release if the IR logic
        changes. A Kepler receiver is a mapping receiver
        with a fixed 21-entry table, but the templates copy the mapping code.
        On RP, `ir_keplers!` can expand into a hidden `ir_mappings!` group
        that has the same member names, plus a thin public group whose `new`
        supplies `KEPLER_MAPPING`. The cost is that Kepler types also
        implement `IrMapping<KeplerKeys>`. On ESP, `ir_kepler!` can't reuse
        `ir_mapping!` directly, because both would generate an inherent `new`
        on the same type. It needs a hidden shared schema with a private
        constructor, which saves little. Decide: do RP only (Kepler gains
        `IrMapping<KeplerKeys>` on RP alone, an API difference between the
        chips), do both, or skip.
  - [x] Delete dead code: core's `IrKeplerStatic` and `IrMappingStatic` were
        public (hidden) but unused; generated code uses `__IrStatic`. Deleted
        2026-11-05.
  - [x] Move `__build_button_map` to core. The RP and ESP `ir/mapping.rs`
        files were byte-identical, which the 2026-10-23 token-level scan
        missed. Moved 2026-11-05; both chips re-export core's version.
- [x] Review for code smells: duplication between the RP and ESP schemas,
      template length, hidden helpers that could become templates (the survey
      names `led2d_from_strip!`), and doc text repeated across schemas.
      Covered by the 2026-11-05 organization review below: RP/ESP template
      duplication is intentional (§2b), `led2d_from_strip!` is already
      template text, and repeated field docs stay inline.
- [x] Add host tests of what the macros expand to:
      `crates/device-envoy-core/tests/macro_expansion.rs`. They read each
      schema's source file, expand calls with `expand_definition`, and check
      the output or the error, without compiling the RP or ESP HALs. They
      cover this release's behavior changes (DMA defaults by member index,
      member visibility, the 0.1 member colon, RP IR `pio`, ESP `led2d!` `len`,
      `pcm_clip!`'s one spelling). The file is excluded from the published
      package because it reads the RP and ESP sources.
- [x] Extend `macro_expansion.rs` to the macros it does not cover yet, as the
      review finds behavior worth pinning down. Added 2026-11-05: ESP
      `button_watch!` spawns one task per declaration, and RP `ir_mapping!`
      takes `button_map` before `pio`. Both tests were confirmed to fail on the
      old code. The API parity test (§2c) covers trait impls and return types
      for every sampled macro.
- [x] RP `led_strips!` keeps `#[allow(clippy::too_many_arguments)]` on the
      generated `new`, approved 2026-10-22, with a comment giving the reason.
- [x] Make generated code lint-clean in a strict user crate. New compile-only
      tests `device-envoy-rp/tests-compile-only/generated_code_lints.rs` and
      `device-envoy-esp/tests/embedded/generated_code_lints_compile.rs` deny
      warnings and `missing_docs`. They found undocumented generated items in
      `led!` (RP, ESP) and `servo_player!` (ESP), which users who deny
      `missing_docs` could not build; now documented. Removed suppressions the
      macros did not need: `led!`'s `non_upper_case_globals` and
      `non_camel_case_types`, `led2d_from_strip!`'s `non_snake_case` and
      `dead_code`, `missing_docs` on RP `__led_strip_type!` and
      `servo_player!` (documented instead), and `pcm_clip!`'s unused
      `SourcePcmClip` alias with its `dead_code` allow.
- [x] `#[allow(non_snake_case)]` on the modules that `pcm_clip!` and
      `adpcm_clip!` generate: approved 2026-10-22, commented. It goes away
      when the modules become structs (inherent associated types).
- [x] `too_many_arguments` on hand-written public constructors
      (`WifiAutoRp::new`, ESP CYD `new`, six RP CYD constructors): approved
      2026-10-22 for this release, commented.
- Decided against (2026-11-05): grouping those constructors' arguments into
  structs (pins, presentation, calibration). Flat arguments, grouped with
  comments in the doc examples, read better, so the `too_many_arguments`
  approvals are permanent.
- [x] Generated code no longer requires users to depend on `paste`,
      `static_cell`, `heapless`, or `embassy_sync`: it reaches them through
      hidden `$crate::__*` re-exports (2026-10-23). The starters dropped those
      dependencies.
- [x] RP `led_strips!` has no hand-written `macro_rules!` left (2026-10-23).
      `led2d_from_strip!` is template text in `led_strips!`, and
      `__led_strip_type!` is a hidden `define!` that the template calls once
      per strip with the name each branch needs (see the survey).
- [x] Generated doc links name `device_envoy_rp::` / `device_envoy_esp::`
      instead of `crate::`, which in generated code is the user's crate; a host
      test rejects `crate::` links in generated docs.
- [x] CI coverage (2026-10-24): `cargo check-all` runs the RP compile-only
      and ESP embedded suites. CI runs it as parallel jobs (`check-core`,
      `check-rp`, `check-esp riscv`, `check-esp xtensa`, `check-docs`,
      `check-msrv`), each on its own runner, still on every push; the duplicate
      `pull_request` run is skipped except for forks. CI now also checks
      formatting and builds the docs.
- [x] Code smell: RP `led!` generated `$decl.vis fn new`, while ESP `led!` and
      ESP `servo_player!` generated `pub fn new`. Resolved by the 2026-10-23
      method-visibility decision (§2b): every generated method is `pub`.
      Confirmed 2026-11-05: no template uses `$decl.vis` on a method.
- Outside this release's scope: about 250 other suppressions, mostly
      `dead_code`, `unused_imports`, and `clippy::future_not_send` in examples
      and compile-only tests.

## 2b. Getting the macros right (decided 2026-10-23)

Work in this order; each step is verified on both chips before the next.

- [x] **ESP LED engine by type.** Done 2026-10-23. `engine: Rmt` / `engine: Spi` (types in
      `led_strip`), chosen through `LedEngine` / `EngineResource` traits. Spike
      proved it on C6, ESP32, and C2 (`esp-engine-trait-spike`). Remaining:
      `led2d!`, delete the 14 dispatch `macro_rules!`, one shared device loop,
      remove `enum Engine`, and update every `engine: Engine::X` call site.
- [x] **LED write errors.** Done 2026-10-23. Recover the RMT channel when a transmission fails
      to start (today one failure kills the strip silently and permanently),
      and report write failures instead of `let _ = driver.write(..)`.
- [x] **`init_and_start!(p)` keeps one arm**; shared peripherals come from
      `new_rmt80(p.RMT)?` and `new_ledc(p.LEDC)`, so callers choose `?` or
      `.expect`. No permutation list, no `.expect` inside (2026-10-23).
- [x] **`combine!` defined once** (2026-10-23) in core and re-exported (RP and ESP copies
      are identical).
- [x] **`impl_wifi_pio`**: `paste!` removed; the task names are passed in
      (2026-10-23). `paste` is no longer a dependency of any Device Envoy crate.
- [x] **RP Wi-Fi spawn handling** (2026-10-24): spawns in `Result`
      functions and the constructor path propagate `Error::TaskSpawn`; the four
      inside never-returning tasks keep `unwrap!`, approved, with a message and
      a comment giving the invariant.
- [x] **Method visibility** (2026-10-23): generated methods are `pub` everywhere; the
      declared type carries the user's visibility (RP `led!` is the outlier).
- **RP and ESP keep separate `define!` schemas and templates, with no shared
  macro code** (decided 2026-10-23). Each chip's template is that chip's API
  in its HAL's terms. Plain runtime logic is shared through core as ordinary
  Rust, as it already is for `LedStrip`, `Frame1d`, and the servo loop.
- [x] `i2cs!`/`lcd_text!`: the duplicated runtime helpers and the I2C task loop
      moved to core (`__run_i2cs`, `__write_text`).
- [x] Fix ESP `i2cs!`/`lcd_text!` doc drift (lost docs on `WIDTH`, `HEIGHT`,
      `ADDRESS`, and both `new`s), and declare LCDs in both strict lint tests.
- [x] Schema parity test (`rp_and_esp_macros_document_the_same_contract`): for every macro both chips define, compare the
      generated Syntax and Fields documentation; intentional differences go on
      an explicit allow-list.
- [x] Remaining families: identical runtime code moved to core (the LED
      runtime, generic over `embedded-hal` `OutputPin`; `LedStripHandle`;
      `AudioPlayerHandle`). A token-level scan finds no identical runtime items
      left in any paired RP/ESP module (2026-10-23).
- [x] LCD write errors are reported, not dropped: core's I2C task takes a
      chip-supplied `report_error` (RP `defmt::warn!`, ESP `log::warn!`), and a
      display whose initialization fails is retried on its next frame
      (2026-10-23).

## 2c. RP/ESP API alignment (decided 2026-10-25)

A survey of the generated APIs of all 15 macro families both chips define found
these differences that the HALs do not explain. HAL-driven differences (which
resources a constructor takes, `impl Into<Peri<..>>` on RP versus by-value
peripherals on ESP, per-member IR constructors on ESP, servo setup types) stay.

- [x] 1. `led2d!` and `led!` return `&'static Self` on both chips; `Led2d` is
      implemented on the panel type itself.
- [x] 2. Panels implement only `Led2d` (one type, one role) and provide
      `strip()`, returning the underlying strip, which implements `LedStrip`.
      ESP moves to RP's structure: a hidden strip plus a panel wrapper.
- [x] 3. ESP `servo_player!` matches RP: the declared type wraps the handle,
      `new` returns `&'static Self`, and it implements `Servo` and `ServoPlayer`.
- [x] 4. ESP `servo!`'s `<Name>Static` struct and `new_static()` become private.
- [x] 5. Arguments go from most specific to least: the device's own pins, then
      its peripherals or channels, then its configuration data (such as
      `on_level` or an IR `button_map`; added 2026-11-05), then shared
      resources, then the spawner. Schemas list fields the same way. Group
      declarations keep the shared field first; group constructors take each
      member's arguments first and the shared resource last. Applies to
      hand-written constructors too.
      Done for every macro-generated constructor (2026-10-25). On 2026-11-05,
      RP `ir_mapping!` changed from `(pin, pio, button_map, spawner)` to
      `(pin, button_map, pio, spawner)`, and its schema fields were reordered
      to match; it was the only generated constructor that broke the
      configuration rule.
      Hand-written constructors, 2026-11-05: a multi-instance device's own
      `&'static …Static` comes first and the spawner last, per the
      Device/Static Pair Pattern in `AGENTS.md`. `RfidEsp::new` and
      `RfidRp::new_spi0/1` now take their pins before the SPI peripheral and
      DMA channels (CHANGELOG has before/after). The hidden `ServoEsp::new`
      and `SpiWs2812::new` take their pin before the shared peripheral.
      Accepted exception (decided 2026-11-05): the four CYD constructors (ESP
      `Cyd::new`, three on RP) keep their current order, with `spi` before
      the pins and `display_spi_hz` among the pins. Reordering would break
      about 300 call sites across device-envoy, cyd-starter, and
      linkage-blaze for a small consistency gain.
- [x] 6. RP macros take `impl Into<Peri<'static, X>>` everywhere (`led!`,
      `lcd_text!`, `i2cs!` take plain `Peri` today).
- [x] 7. ESP `button_watch!`'s `button_pin` parameter is named `pin`.
- [x] 8. ESP `COMBO_TABLE` becomes private, as on RP.
- [x] 9. ESP `engine` is a defaulted field shown as "RMT where the chip has it,
      otherwise SPI".
- [x] Extend the parity test to generated APIs (constructor signatures, return
      types, trait impls), so these differences cannot come back unnoticed.
      Done 2026-11-05: `rp_and_esp_samples_generate_the_same_api` expands each
      chip's sample doc pages, including nested macro calls, and compares each
      generated type's trait impls, public methods (async or not, and return
      type), and public constants. Parameter lists are not compared, since
      they name HAL types. Hidden types (RP's one-member groups) may exist on
      one chip. The one allowed difference is `servo!`'s `new`: it is
      infallible on RP and returns `Result` on ESP (LEDC setup can fail). A
      mutation check confirmed the test fails on a changed return type.

## 3. Branch health

- [x] `cargo fmt --all -- --check` (2026-10-22, at `3d190d09`; rerun
      2026-11-05 at `c7bfaa60`: clean. Rerun before the release PR if more
      commits land.)
- [x] `cargo check-all` passes on the branch (2026-10-22, at `3d190d09`;
      2026-11-05 at `c7bfaa60`).
- [x] Resolve or defer any `TODO0` that appears. Swept 2026-11-05 at
      `c7bfaa60`: Device Envoy's code has none. The only matches are this
      spec's prose about the starters' markers. The starters' five path
      dependency `TODO0`s are expected and resolved after publication
      (section 4).

## 4. Downstream projects

Each needs a branch that builds against this branch through local path
dependencies (do not commit the path dependencies), then a registry update
after publication. Macro usage as of 2026-10-22:

| Project | Macros used | Branch | Builds against branch | Hardware tested |
| --- | --- | --- | --- | --- |
| `device-envoy-esp-blinky` | `led!`, `led_strip!` | `device-envoy-0.2.0` | [x] c6, s3 | [ ] |
| `device-envoy-rp-blinky` | `led!` | `device-envoy-0.2.0` | [x] pico1, pico2, pico1w, pico2w | [ ] |
| `device-envoy-cyd-starter` | `button_watch!` | `device-envoy-0.2.0` | [x] host, ESP32, wasm | [ ] |
| `linkage-blaze` | `button_watch!` | `track-device-envoy-macro-schema` | [x] `cargo check-all` | [ ] |

`linkage-blaze` passed `cargo check-all` on 2026-10-22 with no source changes.
Its branch patches crates.io with this checkout. Its `device-envoy-*`
requirements are `0.2.0` (2026-10-22): a `[patch]` applies only when the
version matches, so at `0.1.6` it silently used the published 0.1.6.

The three starters needed no source changes. On 2026-10-22 their
`device-envoy-0.2.0` branches were switched to this checkout: each
`device-envoy-*` dependency is `{ version = "0.2.0", path = "../device-envoy/..." }`
under a `# TODO0` comment, the blinky package versions are `0.2.0`, and the
lockfiles are refreshed (`cyd-starter`'s `syn` went from 3.0.4 to 3.0.9, which
`macro-schema` needs). All configurations pass against this checkout.

- [ ] After `device-envoy` 0.2.0 is published, resolve each starter's `TODO0`:
      delete `path` (four dependencies: `esp-blinky`, `rp-blinky`,
      `cyd-starter`, `cyd-starter/wasm`), refresh the lockfile, and confirm
      every Device Envoy entry has a crates.io `source` (release checklist
      step 11). Find them with `rg -n TODO0` in each starter. The Device
      Envoy `TODO0` sweep (release checklist step 2) does not search the
      starters.
- Until then the starters build only beside a `device-envoy` checkout at
  `../device-envoy`. Their only CI, `cyd-starter`'s Pages workflow, runs on
  `main`, so these branches do not break it; do not merge them to `main`
  before the `TODO0`s are resolved.
- The `Check Starters` workflow covers only the two blinky starters, not
  `cyd-starter`. Check it by hand with `just check-all` after temporarily adding
  a `[patch.crates-io]` for `device-envoy-core` and `device-envoy-esp`.
- [x] `Check Starters` workflow passes against this branch (2026-10-22, at
      `43a40225`, run 39860296199). Rerun it at release-checklist step 7.

## 5. Docs and migration notes

- [x] Draft the `0.2.0 (unreleased)` section of `CHANGELOG.md`, with
      before-and-after examples for the group shape and RP `servo!`.
- [ ] Review the changelog draft. Decide whether it is enough as the migration
      guide.
- [x] Check README and module-level examples for old syntax (fields before the
      group name, `Name: { ... }` members, `pio_split!`, `odd`/`even`). None
      found on 2026-10-22.
- [x] `just docs` builds and its snapshot check passes (2026-10-22). Fixed the
      deprecated dashed clippy lint names it warned about in the RP manifests.
      The remaining future-incompatibility warning is `proc-macro-error2`.
      On 2026-11-05, `defmt` 1.2.0 removed one source of it, and the lockfiles
      of device-envoy, rp-blinky, and linkage-blaze were updated (linkage-blaze
      also bumped its `=1.0.1` pin). The other source is `pio-proc` 0.3.0,
      through `pio` 0.3, and that is blocked upstream: `pio-proc` 0.4 needs
      `pio` 0.4, and embassy-rp 0.10.0 (the latest) still requires `pio` 0.3.
      Move to `pio` 0.4 when embassy-rp does.
- [x] Automated docs audit (2026-11-05). Each macro's hand-written summary
      was diffed between RP and ESP, and the docs were scanned for wrong link
      targets, the banned "module-level example" phrasing, types named in
      docs that no longer exist, and uncompiled Markdown with old API
      spellings. Fixed:
  - ESP `led_strip!` had lost its summary, usage-examples link,
    `max_frames` note, and current-limiting section.
  - RP `servo!` had a drifted summary.
  - ESP `servo!` and `servo_player!` had duplicate links.
  - ESP `led2d!` had a second "Fields" heading.
  - `led2d!` linked `led_strip!` to the module on both chips.
  - RP `i2cs!` had a self-contradicting sentence.
  - Link text was off-convention (led2d, `cyd`).
  - The RP demos README named a removed `ServoAnimate`.
  - esp-blinky's C2 DevKitM-1 example used the removed `Engine::Spi`.

  New test `every_macro_page_leads_to_usage_examples` checks every public
  macro in Core, RP, and ESP, reading included `.md` files too. A mutation
  check confirmed that it flags the old ESP `led_strip!`. The remaining RP/ESP
  summary differences are hardware-driven: RP's PIO sharing, and the RP-only
  `led_strips!`.
- [x] CI gaps found 2026-11-05.
  - `Check Clippy` had failed since 2026-10-26 on one `manual_memcpy` in
    core's `lcd_text.rs`, unnoticed because `cargo check-all` did not run
    clippy. Now fixed, and `check-all` runs the workflow's six clippy
    commands; keep the two in sync. `check-all` also names a failing
    parallel check.
  - `Check Starters` cloned the starters' `main`, which uses the 0.1 API. It
    now takes a `starter_branch` input; dispatch it with
    `starter_branch=device-envoy-0.2.0` until the starters merge.
- [ ] Review the generated syntax and field tables on each macro page for
      Core, RP, and ESP (`just show-docs-esp`, `just show-docs-rp`; then
      `just docs` to restore the snapshot).

## 6. Release

Follow [docs/release_checklist.md](../docs/release_checklist.md) from step 1.
Notes specific to this release:

- Version `0.2.0`: the syntax changes are breaking.
- Merge `proc-macro-const-structures` into `main` (or into a `release/0.2.0`
  branch cut from it) through a pull request.
- Crate versions are already `0.2.0` on this branch (2026-10-22), so
  checklist step 3 only needs rechecking.
- After publication, resolve the starters' `TODO0`s (section 4), then switch
  `linkage-blaze` to the registry versions and release it against them.

## Decisions

- Version `0.2.0` (2026-10-22).
- Clean break from the old syntax, with a migration guide and no deprecation
  period (2026-10-22).
