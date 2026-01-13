#!/usr/bin/env bash
set -euo pipefail

cargo docdoc

DOCS_DIR="target/thumbv8m.main-none-eabihf/doc/device_kit/docs"
mkdir -p "${DOCS_DIR}"
cp docs/led2d_graphics.png "${DOCS_DIR}/led2d_graphics.png"
