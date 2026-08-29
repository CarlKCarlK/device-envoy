# Release Checklist

This is the canonical release procedure for the Device Envoy workspace. Other
repository documentation should link here rather than duplicate these steps.

## 1. Prepare the Release Branch

- Pick the target version (for example `0.1.3`).
- Prepare the release on a dedicated feature or release branch, not directly on
  `main`.
- Start with a clean working tree and an up-to-date `main` base.
- Format all code files:

```bash
cargo fmt --all -- --check
```

## 2. Sweep TODO Priorities

- Search for release-blocking TODOs:

```bash
rg -n '(?i)\btodo''0+\b' crates xtask specs docs --glob '!docs/release_checklist.md'
```

- Resolve, explicitly defer, or document why each remaining item is not
  blocking.

## 3. Update Versions

- Bump published crate versions together unless a release intentionally
  diverges:
  - `crates/device-envoy-core/Cargo.toml`
  - `crates/device-envoy-rp/Cargo.toml`
  - `crates/device-envoy-esp/Cargo.toml`
  - `crates/device-envoy/Cargo.toml`
- Update version requirements between workspace crates to match.
- Refresh the workspace lockfile through Cargo, for example with
  `cargo update -w` or `cargo check-all`.
- Do not update downstream repositories to an unpublished registry version.
  Validate them against this branch first, then update their registry
  dependencies after publication.

## 4. Update the Changelog

- Update the top-level [CHANGELOG.md](../CHANGELOG.md) with a concise section
  for the release.
- Finalize the release heading before the release pull request. Remove draft
  markers such as `(unreleased)` or `TBD`; the heading in the tagged commit
  must describe a completed release.
- Summarize API changes, behavior changes, and notable fixes.
- Mention downstream repository updates only when those updates are actually
  part of the coordinated release.
- Confirm that no draft marker remains in the release changelog. This command
  must print no matches:

```bash
rg -n -i '\bunreleased\b|\btbd\b' CHANGELOG.md
```

## 5. Generate and Review Documentation

- Build the authoritative Core, ESP, and RP documentation snapshot:

```bash
just docs
```

This is the only authoritative documentation workflow. It rebuilds all three
sites with their complete review feature sets, stages shared images, and
verifies sentinel pages, images, and selected rendered links. Do not use an
individual `*-only` or explicitly incomplete preview for release review.

- Optionally rebuild and open all three sites in a browser:

```bash
just show-docs
```

- Manually inspect Core, RP, and ESP for broken links, stale examples, missing
  images or sections, and inconsistent platform documentation.
- Optionally export rustdoc to DOCX with
  `scripts/rustdoc_site_to_docx.py` for a whole-site comparison.

## 6. Run Full Local Checks

Run the workspace's local CI equivalent from the repository root:

```bash
cargo check-all
```

Fix every failure before pushing the final release-preparation commit.

## 7. Validate the Starter Repositories

The `Check Starters` workflow validates every supported RP and ESP starter
configuration against this Device Envoy branch by substituting local path
dependencies. It is intentionally manual and does not run automatically for a
pull request.

- Push the release branch, then dispatch the workflow against that branch:

```bash
RELEASE_BRANCH="$(git branch --show-current)"
git push -u origin "$RELEASE_BRANCH"
gh workflow run check-starters.yml --ref "$RELEASE_BRANCH"
STARTER_RUN_ID="$(gh run list --workflow check-starters.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
```

- Watch the run identified by the final command and require it to pass:

```bash
gh run watch "$STARTER_RUN_ID" --exit-status
```

- Perform any release-specific hardware smoke tests that cannot run in CI.
- Do not commit temporary path dependencies to a downstream repository.

## 8. Integrate Through a Pull Request

- Commit all release-preparation changes locally. Keep the commit unpushed
  until the package preflight below passes, so any packaging fix can be added
  before opening the pull request.
- Run the Core publish dry-run from the release branch before opening the pull
  request. This verifies the actual archive in a clean target directory and
  catches package-boundary errors that workspace builds cannot see:

```bash
CARGO_TARGET_DIR="$(mktemp -d)" cargo publish --dry-run --locked -p device-envoy-core
```

- Confirm the RP and ESP archive file lists contain every required source,
  documentation, and asset file. Their full dry-runs must wait until the new
  Core version has been published to crates.io:

```bash
cargo package --list -p device-envoy-rp
cargo package --list -p device-envoy-esp
```

- Push the validated release-preparation commit.
- Open or update a pull request from the release branch into `main`.
- Wait for the pull request's `Check All` and `Check Clippy` workflows to pass.
  From the checked-out release branch, inspect their current state with:

```bash
gh pr checks
```

  Repeat the command as needed or monitor the checks in the GitHub pull-request
  page until every required check passes.

- Review the complete pull request diff and confirm that the changelog,
  manifests, and lockfile describe the intended release. In particular,
  confirm that the changelog heading no longer contains an `unreleased` or
  `TBD` marker.
- Merge through GitHub only after the pull-request checks and the manually
  dispatched starter checks pass. Do not bypass this gate with a direct local
  merge and push to `main`.
- Wait for the push-triggered `Check All` and `Check Clippy` workflows on
  `main` to pass after the merge.
- Update the local checkout to the approved commit and confirm it is clean:

```bash
git switch main
git pull --ff-only origin main
git status --short --branch
```

## 9. Publish in Dependency Order

Publishing is effectively permanent. Run every command from the clean,
CI-approved `main` commit. Use a fresh Cargo target directory for each dry run
so one package cannot accidentally rely on files left by another package.

- Dry-run and publish Core first:

```bash
CARGO_TARGET_DIR="$(mktemp -d)" cargo publish --dry-run --locked -p device-envoy-core
cargo publish --locked -p device-envoy-core
```

- Wait until the exact Core version resolves from crates.io:

```bash
cargo info device-envoy-core@X.Y.Z
```

- Dry-run RP and ESP against the published Core version:

```bash
CARGO_TARGET_DIR="$(mktemp -d)" cargo publish --dry-run --locked -p device-envoy-rp
CARGO_TARGET_DIR="$(mktemp -d)" cargo publish --dry-run --locked -p device-envoy-esp
```

- Publish the platform crates; neither depends on the other:

```bash
cargo publish --locked -p device-envoy-rp
cargo publish --locked -p device-envoy-esp
```

- Wait until both exact platform versions resolve from crates.io:

```bash
cargo info device-envoy-rp@X.Y.Z
cargo info device-envoy-esp@X.Y.Z
```

- Do not publish the top-level `device-envoy` landing crate.
- Do not publish the blinky repositories; they are cloneable project
  templates, not crates.io packages.

## 10. Tag and Create the GitHub Release

After all three crates have been published, tag the exact approved `main`
commit:

```bash
git status --short --branch
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

- Create a GitHub Release from the tag using the curated changelog section.
  Do not include a draft changelog marker in the release title or notes, and do
  not use automatically generated notes in place of the curated section. For
  example, after copying that section's body into a temporary Markdown file:

```bash
gh release create vX.Y.Z \
  --title "Device Envoy X.Y.Z" \
  --notes-file /path/to/release-notes.md
```

- Verify that the release uses the intended tag and title and is neither a
  draft nor a prerelease:

```bash
gh release view vX.Y.Z \
  --json name,tagName,isDraft,isPrerelease,url
```

- Add a `release` label to the pull request or tracking issue when useful.

## 11. Verify and Update Downstream Repositories

- Verify the exact Core, RP, and ESP versions on crates.io.
- Verify the Core, RP, and ESP builds on docs.rs.
- Confirm that the top-level README badges resolve to the new versions.
- Update `device-envoy-rp-blinky` and `device-envoy-esp-blinky` to the published
  registry versions in separate pull requests.
- Refresh only the Device Envoy entries in each starter lockfile and review the
  diff to avoid unrelated dependency upgrades.
- Confirm every Device Envoy lockfile entry has a crates.io `source` and
  `checksum`, then verify the lockfile without modifying it:

```bash
cargo metadata --locked --format-version 1 > /dev/null
```

- Validate every supported starter configuration against the published crates:

```bash
# In device-envoy-rp-blinky:
cargo xtask check --board pico1
cargo xtask check --board pico2
cargo xtask check --board pico1w
cargo xtask check --board pico2w

# In device-envoy-esp-blinky:
cargo xtask check --chip c6
cargo xtask check --chip s3
```

- Merge the starter updates only after their pull-request checks pass.
- For a coordinated downstream crate release, such as Linkage Blaze, wait for
  all required Device Envoy versions to resolve from crates.io before running
  registry-only checks and publishing that crate.
