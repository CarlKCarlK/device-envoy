#!/usr/bin/env bash
set -euo pipefail

# Ensure rustdoc output is fresh; stale `type.impl` artifacts can survive
# across runs and show removed items.
WORKSPACE_TARGET="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/target"
rm -rf "${WORKSPACE_TARGET}/doc"
rm -rf "${WORKSPACE_TARGET}/thumbv8m.main-none-eabihf/doc"
rm -rf "${WORKSPACE_TARGET}/thumbv6m-none-eabi/doc"

bash "$(dirname "$0")/../../../scripts/update-doc-images.sh"
cargo xtask check-docs
cargo doc \
    --target thumbv8m.main-none-eabihf \
    --no-deps \
    --features pico2,arm,wifi,doc-images \
    --no-default-features

DOCS_DIR="${WORKSPACE_TARGET}/thumbv8m.main-none-eabihf/doc/device_envoy_rp/docs/assets"
mkdir -p "${DOCS_DIR}"
cp "$(dirname "$0")/../../device-envoy-core/docs/assets/"*.png "${DOCS_DIR}/"
bash "$(dirname "$0")/../../../scripts/check-rendered-cyd-links.sh" \
  "$WORKSPACE_TARGET/thumbv8m.main-none-eabihf/doc/device_envoy_rp/cyd"
