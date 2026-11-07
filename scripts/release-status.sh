#!/usr/bin/env bash
# Read-only report of release state across Device Envoy and the repositories that use it:
# versions, dependency requirements, lockfile sources, CI pins, release TODOs, and each
# repository's git state. It changes nothing. It exits with 1 when it finds a problem.
#
# Sibling repositories default to this layout; override with environment variables:
#   ../device-envoy-esp-blinky, ../device-envoy-rp-blinky, ../device-envoy-cyd-starter
#   ../../linkage-blaze (LINKAGE_BLAZE_DIR), ../../macro-schema (MACRO_SCHEMA_DIR)
set -uo pipefail

DE="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
STARTERS_DIR="${STARTERS_DIR:-$(dirname "$DE")}"
LINKAGE_BLAZE_DIR="${LINKAGE_BLAZE_DIR:-$(dirname "$(dirname "$DE")")/linkage-blaze}"
MACRO_SCHEMA_DIR="${MACRO_SCHEMA_DIR:-$(dirname "$(dirname "$DE")")/macro-schema}"

problems=()
problem() { problems+=("$1"); echo "  ⚠ $1"; }

# The newest version of a crate on crates.io, or "?" when offline.
crates_io_version() {
    curl -fsS -A "device-envoy-release-status" "https://crates.io/api/v1/crates/$1" 2>/dev/null |
        python3 -c 'import json,sys; print(json.load(sys.stdin)["crate"]["max_version"])' 2>/dev/null ||
        echo "?"
}

# The `version = "..."` of a manifest's [package] (or [workspace.package]).
package_version() {
    awk '/^\[(workspace\.)?package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/[" ]/,"",$0); sub(/version=/,""); print; exit}' "$1"
}

# Every line in a manifest that requires one of the Device Envoy crates or macro-schema.
requirements() {
    grep -nE '^(device-envoy-(core|rp|esp)|macro-schema(-core)?) *=' "$1" 2>/dev/null
}

# Branch, cleanliness, and sync with origin.
git_state() {
    local dir="$1" name="$2"
    git -C "$dir" fetch -q origin 2>/dev/null
    local branch commit dirty ahead behind
    branch="$(git -C "$dir" branch --show-current)"
    commit="$(git -C "$dir" rev-parse --short=10 HEAD)"
    dirty="$(git -C "$dir" status --porcelain --untracked-files=no | wc -l)"
    if ! git -C "$dir" rev-parse -q --verify "origin/${branch}" >/dev/null; then
        echo "  git: ${branch} at ${commit}, ${dirty} modified, not on GitHub"
        problem "${name}: branch ${branch} is not on GitHub"
    else
        read -r behind ahead < <(git -C "$dir" rev-list --left-right --count "origin/${branch}...HEAD")
        echo "  git: ${branch} at ${commit}, ${dirty} modified, ${ahead} ahead / ${behind} behind origin"
        [ "$ahead" = 0 ] || problem "${name}: ${ahead} unpushed commits on ${branch}"
    fi
    [ "$dirty" -eq 0 ] || problem "${name}: uncommitted changes"
}

# Release-priority TODOs, outside specs.
todo0() {
    local dir="$1" name="$2" count
    count="$(git -C "$dir" grep -nE '\bTODO0+\b' -- ':!specs' ':!docs/release_checklist.md' 2>/dev/null | wc -l)"
    [ "$count" -eq 0 ] || problem "${name}: ${count} release-priority TODO line(s) (see the sweep in docs/release_checklist.md)"
}

# Each Device Envoy lockfile entry and its source.
lock_sources() {
    local lock="$1" name="$2"
    [ -f "$lock" ] || return
    local line
    # Read from a process substitution, not a pipe, so `problem` runs in this shell.
    while read -r line; do
        echo "  lock: $line"
        case "$line" in *"(local path)"*) problem "${name}: Cargo.lock uses a local Device Envoy (${line%% *})" ;; esac
    done < <(python3 -c '
import re, sys
text = open(sys.argv[1]).read()
for block in text.split("[[package]]"):
    name = re.search(r"^name = \"(device-envoy-(?:core|rp|esp))\"", block, re.M)
    if not name:
        continue
    version = re.search(r"^version = \"([^\"]+)\"", block, re.M).group(1)
    source = "crates.io" if "registry+" in block else "(local path)"
    print(name.group(1), version, source)
' "$lock")
}

echo "Release status ($(date +%Y-%m-%d))"

echo
echo "== device-envoy (${DE})"
git_state "$DE" device-envoy
todo0 "$DE" device-envoy
latest_tag="$(git -C "$DE" tag --list 'v*' --sort=-v:refname | head -1)"
echo "  latest tag: ${latest_tag}"
for crate in device-envoy-core device-envoy-rp device-envoy-esp; do
    local_version="$(package_version "$DE/crates/$crate/Cargo.toml")"
    published="$(crates_io_version "$crate")"
    echo "  ${crate}: ${local_version} here, ${published} on crates.io"
done
echo "  requires: $(requirements "$DE/crates/device-envoy-core/Cargo.toml" | grep macro-schema | cut -d: -f2- | tr '\n' ' ')"

for starter in device-envoy-esp-blinky device-envoy-rp-blinky device-envoy-cyd-starter; do
    dir="${STARTERS_DIR}/${starter}"
    echo
    echo "== ${starter} (${dir})"
    [ -d "$dir" ] || { problem "${starter}: not found at ${dir}"; continue; }
    git_state "$dir" "$starter"
    todo0 "$dir" "$starter"
    for manifest in "$dir/Cargo.toml" "$dir/wasm/Cargo.toml"; do
        [ -f "$manifest" ] || continue
        while IFS= read -r line; do
            echo "  ${manifest#"$dir"/}:${line}"
            case "$line" in *path*) problem "${starter}: path dependency on Device Envoy in ${manifest#"$dir"/}";; esac
        done < <(requirements "$manifest")
    done
    lock_sources "$dir/Cargo.lock" "$starter"
done

echo
echo "== linkage-blaze (${LINKAGE_BLAZE_DIR})"
if [ -d "$LINKAGE_BLAZE_DIR" ]; then
    git_state "$LINKAGE_BLAZE_DIR" linkage-blaze
    todo0 "$LINKAGE_BLAZE_DIR" linkage-blaze
    echo "  linkage-blaze: $(package_version "$LINKAGE_BLAZE_DIR/Cargo.toml") here, $(crates_io_version linkage-blaze) on crates.io"
    for manifest in "$LINKAGE_BLAZE_DIR"/crates/*/Cargo.toml; do
        while IFS= read -r line; do
            echo "  ${manifest#"$LINKAGE_BLAZE_DIR"/}:${line}"
            case "$line" in *path*) problem "linkage-blaze: path dependency on Device Envoy in ${manifest#"$LINKAGE_BLAZE_DIR"/}";; esac
        done < <(requirements "$manifest")
    done
    if grep -q '^\[patch.crates-io\]' "$LINKAGE_BLAZE_DIR/Cargo.toml"; then
        echo "  [patch.crates-io]: builds against ../mcu/device-envoy (expected)"
    fi
    pin_file="$LINKAGE_BLAZE_DIR/.github/device-envoy-ref"
    if [ -f "$pin_file" ]; then
        pin="$(tr -d '[:space:]' < "$pin_file")"
        echo "  CI device-envoy pin: ${pin}"
        [ "$pin" = "$latest_tag" ] || problem "linkage-blaze: CI pins device-envoy ${pin}, latest tag is ${latest_tag} (.github/device-envoy-ref)"
    else
        echo "  CI device-envoy pin: $(grep -h 'ref: v' "$LINKAGE_BLAZE_DIR"/.github/workflows/*.yml | sort -u | tr -s ' ' | tr '\n' ' ')"
    fi
else
    problem "linkage-blaze: not found at ${LINKAGE_BLAZE_DIR}"
fi

echo
echo "== macro-schema (${MACRO_SCHEMA_DIR})"
if [ -d "$MACRO_SCHEMA_DIR" ]; then
    git_state "$MACRO_SCHEMA_DIR" macro-schema
    echo "  macro-schema: $(package_version "$MACRO_SCHEMA_DIR/Cargo.toml") here, $(crates_io_version macro-schema) on crates.io"
else
    problem "macro-schema: not found at ${MACRO_SCHEMA_DIR}"
fi

echo
if [ "${#problems[@]}" -eq 0 ]; then
    echo "No problems found."
else
    echo "${#problems[@]} problem(s):"
    printf '  - %s\n' "${problems[@]}"
    exit 1
fi
