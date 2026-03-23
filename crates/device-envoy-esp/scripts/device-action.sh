#!/usr/bin/env bash
set -euo pipefail

action="${1:-}"
name="${2:-}"
chip="${3:-c6}"

if [[ -z "$action" || -z "$name" ]]; then
  echo "usage: scripts/device-action.sh <run|check|build> <name> [chip]" >&2
  exit 1
fi

case "$action" in
  run|check|build) ;;
  *)
    echo "invalid action '$action' (expected: run, check, build)" >&2
    exit 1
    ;;
esac

has_example=0
if [[ -f "examples/${name}.rs" ]]; then
  has_example=1
fi

demo_candidates="$(find demos -maxdepth 2 -type f \( -name "${name}.rs" -o -name "${name}_*.rs" \) | sort)"
demo_count="$(printf "%s\n" "$demo_candidates" | sed '/^$/d' | wc -l)"
has_demo=0
if [[ "$demo_count" -gt 0 ]]; then
  has_demo=1
fi

if [[ "$has_example" -eq 1 && "$has_demo" -eq 1 ]]; then
  echo "name '$name' is ambiguous (matches both example and demo)" >&2
  exit 1
fi

if [[ "$has_example" -eq 0 && "$has_demo" -eq 0 ]]; then
  echo "unknown name '$name' (no matching example or demo)" >&2
  exit 1
fi

cargo_bin=(cargo)
build_std_args=()
target=""
feature=""

case "$chip" in
  c6)
    target="riscv32imac-unknown-none-elf"
    feature="esp32c6"
    ;;
  c2)
    target="riscv32imc-unknown-none-elf"
    feature="esp32c2"
    ;;
  c3)
    target="riscv32imc-unknown-none-elf"
    feature="esp32c3"
    ;;
  h2)
    target="riscv32imac-unknown-none-elf"
    feature="esp32h2"
    ;;
  esp32)
    target="xtensa-esp32-none-elf"
    feature="esp32"
    cargo_bin=(cargo +esp)
    build_std_args=(-Zbuild-std=core,alloc)
    ;;
  s2)
    target="xtensa-esp32s2-none-elf"
    feature="esp32s2"
    cargo_bin=(cargo +esp)
    build_std_args=(-Zbuild-std=core,alloc)
    ;;
  s3)
    target="xtensa-esp32s3-none-elf"
    feature="esp32s3"
    cargo_bin=(cargo +esp)
    build_std_args=(-Zbuild-std=core,alloc)
    ;;
  *)
    echo "invalid chip '$chip' (expected one of: c6, c2, c3, h2, esp32, s2, s3)" >&2
    exit 1
    ;;
esac

release_args=(--release)

if [[ "${#build_std_args[@]}" -gt 0 ]]; then
  # Required for Xtensa builds.
  source "$HOME/export-esp.sh"
fi

if [[ "$has_example" -eq 1 ]]; then
  "${cargo_bin[@]}" "$action" \
    --example "$name" \
    --target "$target" \
    "${release_args[@]}" \
    --no-default-features \
    --features "$feature" \
    "${build_std_args[@]}"
else
  demo_path="$(printf "%s\n" "$demo_candidates" | sed '/^$/d' | head -n1)"
  demo_stem="$(basename "$demo_path" .rs)"
  demo_bin="demo_${demo_stem}"
  "${cargo_bin[@]}" "$action" \
    --package device-envoy-esp-demos \
    --bin "$demo_bin" \
    --target "$target" \
    "${release_args[@]}" \
    --no-default-features \
    --features "$feature" \
    "${build_std_args[@]}"
fi
