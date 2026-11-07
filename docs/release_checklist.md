# Release Checklist

This is the canonical release procedure for the Device Envoy workspace. Other
repository documentation should link here rather than duplicate these steps.

## 1. Prepare the Release Branch

- Pick the target version (for example `0.1.3`).
- Prepare the release on a dedicated feature or release branch, not directly on
  `main`.
- Start with a clean working tree and an up-to-date `main` base.
- See where every repository stands. `just release-status` reports versions,
  Device Envoy requirements, lockfile sources, Linkage Blaze's CI pin,
  `TODO0`s, and git state across Device Envoy, the starters, Linkage Blaze,
  and macro-schema. Run it again at each later step; at the end of the release
  it should report no problems.
- Format all code files:

```bash
cargo fmt --all -- --check
```

## 2. Sweep TODO Priorities

- Search for release-blocking TODOs:

```bash
rg -n '(?i)\btodo''0+\b' crates xtask docs --glob '!docs/release_checklist.md'
```

  Planning documents in `specs/` carry a `TODO0` by design, so the sweep
  leaves them out. Delete a spec once its work ships, moving anything still
  useful into `docs/`.

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
- When macros changed, open each changed macro's page on both chips side by
  side (`target/<triple>/doc/device_envoy_<chip>/index.html`) and check that:
  - the first line says what the macro generates;
  - a bold link near the top leads to usage examples, or the page has its own
    example;
  - **Syntax** and **Fields** list the fields in constructor order, with
    sensible defaults;
  - the **Related Macros** links go to the right pages;
  - the generated sample type's page (for example `LedStripGenerated`) shows
    `new` with the expected arguments and return type.

  Also check the Core trait and type pages each chip re-exports (`LedStrip`,
  `Led2d`, `Frame1d`, `Gamma`, `ClockSync…`): each starts with a linked "Start
  with the … module documentation" line.
- Optionally export rustdoc to DOCX with
  `scripts/rustdoc_site_to_docx.py` for a whole-site comparison.

## 6. Run Full Local Checks

Run the workspace's local CI equivalent from the repository root. Update
nightly first: `check-all` includes the docs.rs check, and docs.rs always
builds with the newest nightly.

```bash
rustup update nightly
cargo check-all
```

Fix every failure before pushing the final release-preparation commit.

The docs.rs check builds each published crate's documentation as docs.rs does
(see `check_docs_rs` in `xtask/src/main.rs`). It exists because the docs.rs
build of `device-envoy-rp` 0.2.0 failed on new nightly rustdoc lints that
the older local and CI toolchains did not report, and the fix needed a new
release (0.2.1). To run it alone:

```bash
cargo run --manifest-path xtask/Cargo.toml -- check-docs-rs
```

## 7. Validate the Starter Repositories

The `Check Starters` workflow validates every supported RP and ESP starter
configuration against this Device Envoy branch by substituting local path
dependencies. It is intentionally manual and does not run automatically for a
pull request.

- Push the release branch, then dispatch the workflow against that branch. If
  the starters need changes for this release, put them on a branch with the
  same name in each starter and pass it as `starter_branch`; without it, the
  workflow checks the starters' `main`:

```bash
RELEASE_BRANCH="$(git branch --show-current)"
git push -u origin "$RELEASE_BRANCH"
gh workflow run check-starters.yml --ref "$RELEASE_BRANCH" \
  -f starter_branch=STARTER_BRANCH
STARTER_RUN_ID="$(gh run list --workflow check-starters.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
```

- Watch the run identified by the final command and require it to pass:

```bash
gh run watch "$STARTER_RUN_ID" --exit-status
```

- The workflow builds every rp-blinky board and every esp-blinky board
  example. It does not cover `device-envoy-cyd-starter`. Check that locally:
  temporarily add a `[patch.crates-io]` pointing `device-envoy-core` and
  `device-envoy-esp` at this checkout, run `just check-all` there, and remove
  the patch.
- Check Linkage Blaze against this branch with `cargo check-all` in that
  repository. Its permanent `[patch.crates-io]` builds against
  `../mcu/device-envoy`, but a patch applies only when the patched crate's
  version satisfies the requirement. If Linkage Blaze still requires the
  previous version, Cargo silently builds against crates.io instead, so first
  set its Device Envoy requirements to this release's version.
- Run hardware smoke tests for behavior compilation cannot check. The usual
  set:
  - `device-envoy-esp-blinky`: `just run c6` and an S3 board example; the
    LED blinks SOS.
  - `device-envoy-rp-blinky`: `cargo xtask run --board <board>` on an RP2040
    and an RP2350 board, at least one with Wi-Fi; the LED blinks SOS and the
    `defmt` log lines decode.
  - `device-envoy-cyd-starter`: `just run-esp`; the display, touch, and the
    BOOT button work.
  - Linkage Blaze on a CYD: the clock and the other demos run.
  - Anything the release changed at runtime that the starters do not use,
    such as two `button_watch!` buttons on ESP in 0.2.0.
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
- Wait for the pull request's `Check All`, `Check Clippy`, and `Check docs.rs`
  workflows to pass.
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
- Wait for the push-triggered `Check All`, `Check Clippy`, and `Check docs.rs`
  workflows on `main` to pass after the merge.
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
- Update `device-envoy-rp-blinky`, `device-envoy-esp-blinky`, and
  `device-envoy-cyd-starter` (including its `wasm` crate) to the published
  registry versions in separate pull requests.
- Bump each starter package version to the Device Envoy release version so a
  cloned starter clearly identifies the library release it targets.
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

# In device-envoy-esp-blinky: src/main.rs, then every board example
just check c6
for example in $(sed -n 's/^name = "blinky_\(.*\)"$/\1/p' Cargo.toml); do
  just check "${example%%_*}" "${example#*_}"
done

# In device-envoy-cyd-starter: host, ESP32, and WASM
just check-all
```

- The starters have no pull-request checks of their own. After merging the
  starter updates, run `Check Starters` without `starter_branch` so it checks
  their `main`:

```bash
gh workflow run check-starters.yml --ref main
```

- For a coordinated downstream crate release, such as Linkage Blaze, wait for
  all required Device Envoy versions to resolve from crates.io before running
  registry-only checks and publishing that crate.
