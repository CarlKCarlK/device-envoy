#!/usr/bin/env bash
set -e

DOC_PARENT="$(pwd)/target/thumbv8m.main-none-eabihf/doc"
DOC_PATH="$DOC_PARENT/device_kit/index.html"

if [ ! -f "$DOC_PATH" ]; then
  echo "Error: Documentation not found at $DOC_PATH"
  echo "Run 'cargo docdoc' first to build the docs"
  exit 1
fi

# Convert to Windows file URL
WIN_PATH=$(wslpath -w "$DOC_PATH")
FILE_URL="file:///${WIN_PATH//\\/\/}"

echo "$FILE_URL"

# Open in default browser
powershell.exe -NoProfile -Command "Invoke-Item '$WIN_PATH'" &
