#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
DOC_PATH="${WORKSPACE_ROOT}/target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/index.html"

if [ ! -f "$DOC_PATH" ]; then
  echo "Error: Documentation not found at $DOC_PATH"
  echo "Run 'just show-docs-esp' to build and open the docs"
  exit 1
fi

WIN_PATH=$(wslpath -w "$DOC_PATH")
echo "Opening: file:///${WIN_PATH//\\//}"
powershell.exe -NoProfile -Command "Invoke-Item '$WIN_PATH'" &
