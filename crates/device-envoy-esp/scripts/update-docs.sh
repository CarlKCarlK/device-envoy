#!/usr/bin/env bash
set -euo pipefail

source "$HOME/export-esp.sh"
cargo doc --no-deps --release --target riscv32imac-unknown-none-elf --no-default-features
