#!/usr/bin/env bash
set -euo pipefail

cargo xtask check-docs

DOCS_DIR="target/thumbv8m.main-none-eabihf/doc/device_kit/docs/assets"
mkdir -p "${DOCS_DIR}"
cp docs/assets/led2d_graphics.png "${DOCS_DIR}/led2d_graphics.png"
