#!/usr/bin/env bash
set -euo pipefail

# Ensure rustdoc output is fresh; stale `type.impl` artifacts can survive
# across runs and show removed items.
rm -rf target/doc
rm -rf target/thumbv8m.main-none-eabihf/doc
rm -rf target/thumbv6m-none-eabi/doc

cargo xtask check-docs
cargo update-docs --features doc-images

DOCS_DIR="target/thumbv8m.main-none-eabihf/doc/device_envoy/docs/assets"
mkdir -p "${DOCS_DIR}"
cp docs/assets/*.png "${DOCS_DIR}/"
