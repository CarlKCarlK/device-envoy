# Release Checklist

Use this checklist when preparing a new workspace release.

## 1. Prep

- format all code files
- Pick the target version (for example `0.0.5-alpha.6`).
- Create a release branch if needed.

## 2. Sweep TODO Priorities

- Search for release-blocking TODOs:
  - `rg -n "\\bTODO00\\b|\\bTODO0\\b" crates xtask`
- Resolve, defer explicitly, or document why each remaining item is not blocking.

## 3. Update Versions

- Bump versions together (lockstep unless intentionally diverging):
  - `crates/device-envoy-core/Cargo.toml`
  - `crates/device-envoy-rp/Cargo.toml`
  - `crates/device-envoy-esp/Cargo.toml`
  - `crates/device-envoy/Cargo.toml`
- Verify dependency constraints between workspace crates still match the new version.
- Update dependent/sample project dependencies to the new versions (for example `device-envoy-rp-blinky` and `device-envoy-esp-blinky`).
- Refresh the workspace lockfile through Cargo after version bumps (for example `cargo update -w` or `cargo check-all`).

## 4. Update Changelog

- Update top-level [CHANGELOG.md](../CHANGELOG.md) with a new section for the release.
- Summarize API changes, behavior changes, and notable fixes.
- Include a note that `device-envoy-rp-blinky` and `device-envoy-esp-blinky` were updated for this release.

## 5. Generate and Review Docs

- Regenerate docs:

```bash
just update-docs-rp
just update-docs-esp
```

- Optional local preview in a browser:

```bash
just show-docs-rp
just show-docs-esp
```

Note: `show-docs-rp` and `show-docs-esp` run the corresponding `update-docs-*` step before opening.

- Manually inspect docs output for both crates (`rp` and `esp`) for broken links, stale examples, and missing sections.
- Optional: export rustdoc to a single DOCX for whole-site diff/review using `scripts/rustdoc_site_to_docx.py` (for example, export current output and a `main` baseline, then compare the two DOCX files in your diff tool).

## 6. Run Full Checks

- Run top-level checks:

```bash
cargo check-all
```

- Fix failures before publishing.

## 7. Validate Sample Projects

- Update `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to the new release version.
- Temporarily patch their `device-envoy-*` dependencies to this local workspace.
- Run the sample/template projects that depend on this workspace (including `device-envoy-rp-blinky` and `device-envoy-esp-blinky`).
- Confirm they build and run with the new versions.
- Remove the temporary local patches; do not commit machine-specific paths.

## 8. Publish Dry Run

- Run dry-runs in publish order:

```bash
cargo publish-core-dry-run
cargo publish-rp-dry-run
cargo publish-esp-dry-run
# skip cargo publish-device-envoy-dry-run
```

## 9. Publish

- Publish in dependency order:

```bash
cargo publish-core
cargo publish-rp
cargo publish-esp
# skip cargo publish-device-envoy
```

- Wait for crates.io index propagation after publishing `core`/`rp`/`esp`.
- The blinky repositories are cloneable templates and are not published to crates.io.

## 10. Tag and GitHub Release

- Create an annotated git tag:

```bash
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

- Create a GitHub Release from that tag.
- Optionally add a `release` label to the PR/issue used to track the release.

## 11. Post-Release Verification

- Verify crate pages on crates.io for all published crates.
- Verify docs.rs builds for `device-envoy-rp` and `device-envoy-esp`.
- Confirm the top-level README version badges reflect the new release.
- Refresh the blinky repositories' lockfiles from crates.io, validate every
  supported starter configuration, and push their dependency updates:

```bash
# in device-envoy-rp-blinky
cargo xtask check --board pico1
cargo xtask check --board pico2
cargo xtask check --board pico1w
cargo xtask check --board pico2w

# in device-envoy-esp-blinky
cargo xtask check --chip c6
cargo xtask check --chip s3
```
