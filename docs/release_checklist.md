# Release Checklist

Use this checklist when preparing a new workspace release.

## 1. Prep

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

## 4. Update Changelog

- Update top-level [CHANGELOG.md](../CHANGELOG.md) with a new section for the release.
- Summarize API changes, behavior changes, and notable fixes.

## 5. Generate and Review Docs

- Regenerate docs:

```bash
just update-docs-rp
just update-docs-esp
```

- Manually inspect docs output for both crates (`rp` and `esp`) for broken links, stale examples, and missing sections.

## 6. Run Full Checks

- Run top-level checks:

```bash
just check-all
```

- Fix failures before publishing.

## 7. Validate Sample Projects

- Run the sample/template projects that depend on this workspace (for example `device-envoy-rp-blinky` and `device-envoy-esp-blinky`).
- Confirm they build and run with the new versions.

## 8. Publish Dry Run

- Run dry-runs in publish order:

```bash
cargo publish-core-dry-run
cargo publish-rp-dry-run
cargo publish-esp-dry-run
cargo publish-device-envoy-dry-run
```

## 9. Publish

- Publish in dependency order:

```bash
cargo publish-core
cargo publish-rp
cargo publish-esp
cargo publish-device-envoy
```

- Wait for index propagation between publishes if needed.

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
