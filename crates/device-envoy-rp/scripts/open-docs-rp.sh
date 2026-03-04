#!/usr/bin/env bash
set -e

WORKSPACE_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
DOC_PATH="${WORKSPACE_ROOT}/target/thumbv8m.main-none-eabihf/doc/device_envoy_rp/index.html"

if [ ! -f "$DOC_PATH" ]; then
  echo "Error: Documentation not found at $DOC_PATH"
  echo "Run 'just show-docs-rp' to build and open the docs"
  exit 1
fi

WIN_PATH=$(wslpath -w "$DOC_PATH")
echo "Opening: file:///${WIN_PATH//\\/\/}"
powershell.exe -NoProfile -Command "Invoke-Item '$WIN_PATH'" &
