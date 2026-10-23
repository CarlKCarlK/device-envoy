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
      remaining hand-written helpers (`__led_engine_normalize!` and friends).
- [ ] Review for code smells: duplication between the RP and ESP schemas,
      template length, hidden helpers that could become templates (the survey
      names `led2d_from_strip!`), and doc text repeated across schemas.
- [x] Add host tests of what the macros expand to:
      `crates/device-envoy-core/tests/macro_expansion.rs`. They read each
      schema's source file, expand calls with `expand_definition`, and check
      the output or the error, without compiling the RP or ESP HALs. They
      cover this release's behavior changes (DMA defaults by member index,
      member visibility, the 0.1 member colon, RP IR `pio`, ESP `led2d!` `len`,
      `pcm_clip!`'s one spelling). The file is excluded from the published
      package because it reads the RP and ESP sources.
- [ ] Extend `macro_expansion.rs` to the macros it does not cover yet, as the
      review finds behavior worth pinning down.
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
- [ ] Approve or remove `#[allow(non_snake_case)]` on the modules that
      `pcm_clip!` and `adpcm_clip!` generate (`device-envoy-core/src/audio_player.rs`).
- [ ] Decide the `too_many_arguments` suppressions on hand-written public
      constructors: `WifiAutoRp::new` (10 arguments, `allow`), ESP CYD `new`
      (18 arguments, `allow`), and six RP CYD constructors (`expect` with a
      reason, `device-envoy-rp/src/cyd.rs`, `cyd/one_spi.rs`).
- [ ] CI gap: the GitHub `Check All` workflow runs only the top-level
      `cargo check-all`, which does not run RP `check-compile-only` or ESP
      `check-embedded-tests`, where most macro compile tests live. Run them
      locally before release, and decide whether CI or the top-level
      `check-all` should run them.
- [ ] Code smell: RP `led!` generates `$decl.vis fn new`, while ESP `led!` and
      ESP `servo_player!` generate `pub fn new`. Pick one.
- Outside this release's scope: about 250 other suppressions, mostly
      `dead_code`, `unused_imports`, and `clippy::future_not_send` in examples
      and compile-only tests.

## 3. Branch health

- [x] `cargo fmt --all -- --check` (2026-10-22, at `3d190d09`; rerun before
      the release PR)
- [x] `cargo check-all` passes on the branch (2026-10-22, at `3d190d09`).
- [ ] Resolve or defer any `TODO0` that appears (none as of 2026-10-22).

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
Its branch already uses path dependencies on this checkout; after publication
it switches back to registry `0.2.0` versions.

The three starters built unchanged against this branch on 2026-10-22, so their
branches need only the post-publication update: `device-envoy-*` dependency
and package version to `0.2.0`, and a lockfile refresh.

- `cyd-starter`'s lockfile pins `syn` 3.0.4, but `macro-schema` needs 3.0.6 or
  later. Its update needs `cargo update -p syn` as well as the Device Envoy
  entries.
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
      The remaining future-incompatibility warning is `proc-macro-error2`,
      pulled in upstream by `defmt-macros`.
- [ ] Review the generated syntax and field tables on each macro page for
      Core, RP, and ESP (`just show-docs`).

## 6. Release

Follow [docs/release_checklist.md](../docs/release_checklist.md) from step 1.
Notes specific to this release:

- Version `0.2.0`: the syntax changes are breaking.
- Merge `proc-macro-const-structures` into `main` (or into a `release/0.2.0`
  branch cut from it) through a pull request.
- After publication, update the downstream projects in section 4 to the
  registry versions, then release `linkage-blaze` against them.

## Decisions

- Version `0.2.0` (2026-10-22).
- Clean break from the old syntax, with a migration guide and no deprecation
  period (2026-10-22).
