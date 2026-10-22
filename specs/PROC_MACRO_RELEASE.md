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
- [x] Confirm that 0.1.0 is enough for this release (2026-10-22: the only
      `macro-schema` change since publication is a README edit). If Device
      Envoy later needs a framework change, list it here; each one means
      publishing a new `macro-schema` version before Device Envoy can publish.

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
- [ ] `Check Starters` workflow passes against this branch (release checklist
      step 7). Needs the branch pushed.

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
