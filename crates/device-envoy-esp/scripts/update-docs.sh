#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ESP_CRATE_DIR="$(dirname "$SCRIPT_DIR")"
WORKSPACE_ROOT="$(git -C "$ESP_CRATE_DIR" rev-parse --show-toplevel)"
WORKSPACE_TARGET="$WORKSPACE_ROOT/target"
rm -rf "${WORKSPACE_TARGET}/doc"
rm -rf "${WORKSPACE_TARGET}/riscv32imac-unknown-none-elf/doc"
rm -rf "${WORKSPACE_TARGET}/xtensa-esp32s3-none-elf/doc"

source "$HOME/export-esp.sh"
cd "$ESP_CRATE_DIR"
bash "$WORKSPACE_ROOT/scripts/update-doc-images.sh"
cargo run \
  --manifest-path "$WORKSPACE_ROOT/crates/device-envoy-examples-esp/xtask/Cargo.toml" \
  --target x86_64-unknown-linux-gnu \
  -- check-docs

DOCS_DIR="${WORKSPACE_TARGET}/riscv32imac-unknown-none-elf/doc/device_envoy_esp/docs/assets"
mkdir -p "${DOCS_DIR}"
cp "$WORKSPACE_ROOT/crates/device-envoy-core/docs/assets/"*.png "${DOCS_DIR}/"
