#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
LOCK_FILE="${WORKSPACE_ROOT}/target/.doc-images.lock"

mkdir -p "$(dirname "${LOCK_FILE}")"

if command -v flock >/dev/null 2>&1; then
    exec 9>"${LOCK_FILE}"
    flock 9
fi

echo "Regenerating shared docs images into crates/device-envoy-core/docs/assets ..."
DEVICE_KIT_UPDATE_PNGS=1 cargo test \
    --manifest-path "${WORKSPACE_ROOT}/Cargo.toml" \
    --target x86_64-unknown-linux-gnu \
    --features host \
    -p device-envoy-core \
    --test pngs
