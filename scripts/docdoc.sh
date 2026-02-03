#!/usr/bin/env bash
set -euo pipefail

cargo xtask check-docs
cargo update-docs --features doc-images

DOCS_DIR="target/thumbv8m.main-none-eabihf/doc/device_envoy/docs/assets"
mkdir -p "${DOCS_DIR}"
cp docs/assets/*.png "${DOCS_DIR}/"
