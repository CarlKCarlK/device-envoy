#!/usr/bin/env bash
set -euo pipefail

action="${1:-}"
name="${2:-}"
board="${3:-1}"

if [[ -z "$action" || -z "$name" ]]; then
  echo "usage: scripts/device-action.sh <run|check|build> <name> [board]" >&2
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

target=""
features=""
case "$board" in
  1)
    target="thumbv6m-none-eabi"
    features="pico1,arm"
    ;;
  2)
    target="thumbv8m.main-none-eabihf"
    features="pico2,arm"
    ;;
  w)
    target="thumbv6m-none-eabi"
    features="pico1,arm,wifi"
    ;;
  2w)
    target="thumbv8m.main-none-eabihf"
    features="pico2,arm,wifi"
    ;;
  2r)
    target="riscv32imac-unknown-none-elf"
    features="pico2,riscv"
    ;;
  *)
    echo "invalid board '$board' (expected one of: 1, 2, w, 2w, 2r)" >&2
    exit 1
    ;;
esac

release_args=()
if [[ "$action" != "check" ]]; then
  release_args=(--release)
fi

if [[ "$has_example" -eq 1 ]]; then
  cargo "$action" \
    --example "$name" \
    --target "$target" \
    "${release_args[@]}" \
    --features "$features" \
    --no-default-features
else
  demo_path="$(printf "%s\n" "$demo_candidates" | sed '/^$/d' | head -n1)"
  demo_stem="$(basename "$demo_path" .rs)"
  demo_bin="demo_${demo_stem}"
  cargo "$action" \
    --package device-envoy-demos \
    --bin "$demo_bin" \
    --target "$target" \
    "${release_args[@]}" \
    --features "$features" \
    --no-default-features
fi
