#!/usr/bin/env bash
set -euo pipefail

action="${1:-}"
name="${2:-}"
chip="${3:-}"
port_arg="${4:-}"
invocation_dir="${5:-}"
requested_name="${name}"

# Convenience: allow passing a Rust filename like `foo.rs`.
if [[ "$name" == *.rs ]]; then
  name="${name%.rs}"
fi

# If the chip slot contains a serial-port token and no explicit port was given,
# treat it as the port argument so callers can do: `just run blinky USB0`.
if [[ -z "$port_arg" ]]; then
  case "$chip" in
    /dev/*|tty*|USB*|ACM*)
      port_arg="$chip"
      chip=""
      ;;
  esac
fi

user_requested_port="${port_arg}"
port="${port_arg:-${ESPFLASH_PORT:-}}"

# Backward-compatible fallback for unquoted callers that place invocation_dir
# in the chip argument position.
if [[ -z "$invocation_dir" && "$chip" == */* && -d "$chip" ]]; then
  invocation_dir="$chip"
  chip=""
fi

infer_blinky_example_from_invocation_dir() {
  if [[ "$name" != "blinky" || -z "$invocation_dir" ]]; then
    return
  fi

  local relative_invocation_dir="$invocation_dir"
  local workspace_root="$PWD"
  if [[ "$relative_invocation_dir" == "$workspace_root"* ]]; then
    relative_invocation_dir="${relative_invocation_dir#"$workspace_root"/}"
  fi
  if [[ "$relative_invocation_dir" != examples/*/* ]]; then
    return
  fi

  local chip_dir="${relative_invocation_dir#examples/}"
  chip_dir="${chip_dir%%/*}"
  local board_dir="${relative_invocation_dir#examples/${chip_dir}/}"
  board_dir="${board_dir%%/*}"
  local chip_feature=""
  local inferred_chip=""
  case "$chip_dir" in
    esp32)
      chip_feature="esp32"
      inferred_chip="esp32"
      ;;
    c2)
      chip_feature="esp32c2"
      inferred_chip="c2"
      ;;
    c3)
      chip_feature="esp32c3"
      inferred_chip="c3"
      ;;
    c5)
      chip_feature="esp32c5"
      inferred_chip="c5"
      ;;
    c6)
      chip_feature="esp32c6"
      inferred_chip="c6"
      ;;
    c61)
      chip_feature="esp32c61"
      inferred_chip="c61"
      ;;
    h2)
      chip_feature="esp32h2"
      inferred_chip="h2"
      ;;
    s2)
      chip_feature="esp32s2"
      inferred_chip="s2"
      ;;
    s3)
      chip_feature="esp32s3"
      inferred_chip="s3"
      ;;
    *)
      return
      ;;
  esac

  local inferred_example="blinky_${chip_feature}_${board_dir}"
  if grep -Eq "^[[:space:]]*name[[:space:]]*=[[:space:]]*\"${inferred_example}\"[[:space:]]*$" Cargo.toml; then
    name="$inferred_example"
    if [[ -z "$chip" ]]; then
      chip="$inferred_chip"
    fi
  fi
}

infer_board_example_from_invocation_dir() {
  if [[ -z "$invocation_dir" ]]; then
    return
  fi

  local relative_invocation_dir="$invocation_dir"
  local workspace_root="$PWD"
  if [[ "$relative_invocation_dir" == "$workspace_root"* ]]; then
    relative_invocation_dir="${relative_invocation_dir#"$workspace_root"/}"
  fi
  if [[ "$relative_invocation_dir" != examples/*/* ]]; then
    return
  fi

  local chip_dir="${relative_invocation_dir#examples/}"
  chip_dir="${chip_dir%%/*}"
  local board_dir="${relative_invocation_dir#examples/${chip_dir}/}"
  board_dir="${board_dir%%/*}"
  local in_talk1_dir=0
  if [[ "$relative_invocation_dir" == examples/*/*/talk1 || "$relative_invocation_dir" == examples/*/*/talk1/* ]]; then
    in_talk1_dir=1
  fi
  local chip_feature=""
  local inferred_chip=""
  case "$chip_dir" in
    esp32)
      chip_feature="esp32"
      inferred_chip="esp32"
      ;;
    c2)
      chip_feature="esp32c2"
      inferred_chip="c2"
      ;;
    c3)
      chip_feature="esp32c3"
      inferred_chip="c3"
      ;;
    c5)
      chip_feature="esp32c5"
      inferred_chip="c5"
      ;;
    c6)
      chip_feature="esp32c6"
      inferred_chip="c6"
      ;;
    c61)
      chip_feature="esp32c61"
      inferred_chip="c61"
      ;;
    h2)
      chip_feature="esp32h2"
      inferred_chip="h2"
      ;;
    s2)
      chip_feature="esp32s2"
      inferred_chip="s2"
      ;;
    s3)
      chip_feature="esp32s3"
      inferred_chip="s3"
      ;;
    *)
      return
      ;;
  esac

  if [[ ! -f "$invocation_dir/${name}.rs" ]]; then
    return
  fi

  local base_name="$name"
  if [[ "$in_talk1_dir" -eq 1 && "$base_name" != talk1_* ]]; then
    base_name="talk1_${base_name}"
  fi
  local inferred_example="${base_name}_${chip_feature}_${board_dir}"
  if grep -Eq "^[[:space:]]*name[[:space:]]*=[[:space:]]*\"${inferred_example}\"[[:space:]]*$" Cargo.toml; then
    name="$inferred_example"
    if [[ -z "$chip" ]]; then
      chip="$inferred_chip"
    fi
  fi
}

normalize_port() {
  local port_value="${1:-}"
  if [[ -z "$port_value" ]]; then
    printf "%s" ""
    return
  fi
  if [[ "$port_value" == /dev/* ]]; then
    printf "%s" "$port_value"
    return
  fi
  if [[ "$port_value" == tty* ]]; then
    printf "%s" "/dev/$port_value"
    return
  fi
  if [[ "$port_value" == ACM* || "$port_value" == USB* ]]; then
    printf "%s" "/dev/tty$port_value"
    return
  fi
  printf "%s" "$port_value"
}

list_serial_ports() {
  shopt -s nullglob
  local serial_ports=()
  local serial_port=""
  for serial_port in /dev/ttyUSB* /dev/ttyACM*; do
    if [[ -e "$serial_port" ]]; then
      serial_ports+=("$serial_port")
    fi
  done
  local serial_port_value=""
  for serial_port_value in "${serial_ports[@]}"; do
    printf "%s\n" "$serial_port_value"
  done
}

port="$(normalize_port "$port")"
if [[ -n "$user_requested_port" && ! -e "$port" ]]; then
  echo "requested serial port '$port' was not found" >&2
  echo "available serial ports:" >&2
  mapfile -t available_ports < <(list_serial_ports)
  if [[ "${#available_ports[@]}" -eq 0 ]]; then
    echo "  (none)" >&2
  else
    printf "  - %s\n" "${available_ports[@]}" >&2
  fi
  exit 1
fi

if [[ -z "$action" || -z "$name" ]]; then
  echo "usage: scripts/device-action.sh <run|check|build> <name> [chip] [port] [invocation_dir]" >&2
  echo "port can be /dev/ttyUSB0, ttyUSB0, USB0, ACM0, etc." >&2
  exit 1
fi

infer_blinky_example_from_invocation_dir
infer_board_example_from_invocation_dir

if [[ "$requested_name" != "blinky" && "$name" == blinky_esp32* ]]; then
  echo "board blinky examples must be run from their board directory using 'blinky'" >&2
  echo "example: cd crates/device-envoy-esp/examples/c6/devkitc1_n8 && just $action blinky" >&2
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
if [[ "$has_example" -eq 0 ]] && grep -Eq "^[[:space:]]*name[[:space:]]*=[[:space:]]*\"${name}\"[[:space:]]*$" Cargo.toml; then
  has_example=1
fi

if [[ "$has_example" -eq 0 ]]; then
  echo "unknown example '$name' (no matching example)" >&2
  exit 1
fi

cargo_bin=(cargo)
build_std_args=()
target=""
feature=""
if [[ -z "$chip" ]]; then
  chip="c6"
fi

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
  c5)
    target="riscv32imac-unknown-none-elf"
    feature="esp32c5"
    ;;
  h2)
    target="riscv32imac-unknown-none-elf"
    feature="esp32h2"
    ;;
  c61)
    target="riscv32imac-unknown-none-elf"
    feature="esp32c61"
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
    echo "invalid chip '$chip' (expected one of: c6, c5, c61, c2, c3, h2, esp32, s2, s3)" >&2
    exit 1
    ;;
esac

release_args=(--release)

if [[ "${#build_std_args[@]}" -gt 0 ]]; then
  # Required for Xtensa builds.
  source "$HOME/export-esp.sh"
fi

if [[ "$action" == "run" && -z "$port" ]]; then
  mapfile -t detected_ports < <(list_serial_ports)
  if [[ "${#detected_ports[@]}" -gt 1 ]]; then
    echo "multiple serial devices detected; refusing to auto-select a flash port:" >&2
    printf "  - %s\n" "${detected_ports[@]}" >&2
    echo "pass a port explicitly, for example:" >&2
    echo "  just run $name $chip ttyUSB1" >&2
    echo "or" >&2
    echo "  just run $name $chip /dev/ttyUSB1" >&2
    exit 1
  fi
  if [[ "${#detected_ports[@]}" -eq 1 ]]; then
    port="${detected_ports[0]}"
  fi
fi

# When multiple ESP boards are connected, set an explicit serial port to avoid
# espflash interactive selection prompts.
if [[ "$action" == "run" && -n "$port" ]]; then
  export ESPFLASH_PORT="$port"
fi

"${cargo_bin[@]}" "$action" \
  --example "$name" \
  --target "$target" \
  "${release_args[@]}" \
  --no-default-features \
  --features "$feature" \
  "${build_std_args[@]}"
