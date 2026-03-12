#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_TARGET="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/target"
rm -rf "${WORKSPACE_TARGET}/doc"
rm -rf "${WORKSPACE_TARGET}/riscv32imac-unknown-none-elf/doc"
rm -rf "${WORKSPACE_TARGET}/xtensa-esp32s3-none-elf/doc"

source "$HOME/export-esp.sh"
bash "$(dirname "$0")/../../../scripts/update-doc-images.sh"
cargo xtask check-docs

DOCS_DIR="${WORKSPACE_TARGET}/riscv32imac-unknown-none-elf/doc/device_envoy_esp/docs/assets"
mkdir -p "${DOCS_DIR}"
cp "$(dirname "$0")/../../device-envoy-core/docs/assets/"*.png "${DOCS_DIR}/"
