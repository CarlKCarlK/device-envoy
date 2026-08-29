#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CORE_DOCS="${WORKSPACE_ROOT}/target/doc/device_envoy_core"
ESP_DOCS="${WORKSPACE_ROOT}/target/riscv32imac-unknown-none-elf/doc/device_envoy_esp"
RP_DOCS="${WORKSPACE_ROOT}/target/thumbv8m.main-none-eabihf/doc/device_envoy_rp"

required_pages=(
    "${CORE_DOCS}/index.html"
    "${CORE_DOCS}/memory/struct.CydMemory.html"
    "${CORE_DOCS}/wasm/struct.CydWasm.html"
    "${CORE_DOCS}/wasm/struct.CydFrameWasm.html"
    "${ESP_DOCS}/index.html"
    "${ESP_DOCS}/cyd/struct.CydEsp.html"
    "${ESP_DOCS}/cyd/struct.CydFrameEsp.html"
    "${RP_DOCS}/index.html"
    "${RP_DOCS}/cyd/struct.CydRp.html"
    "${RP_DOCS}/cyd/struct.CydFrameRp.html"
)

for required_page in "${required_pages[@]}"; do
    if [[ ! -s "${required_page}" ]]; then
        echo "missing full-documentation sentinel page: ${required_page}" >&2
        exit 1
    fi
done

if ! rg -q 'src="data:image/png;base64,' \
    "${CORE_DOCS}/memory/struct.CydMemory.html"; then
    echo "CydMemory documentation is missing its embedded preview image" >&2
    exit 1
fi

for source_image in "${WORKSPACE_ROOT}/crates/device-envoy-core/docs/assets/"*.png; do
    if [[ ! -s "${source_image}" ]]; then
        echo "missing documentation source image: ${source_image}" >&2
        exit 1
    fi

    image_name="$(basename "${source_image}")"
    for platform_assets in "${ESP_DOCS}/docs/assets" "${RP_DOCS}/docs/assets"; do
        staged_image="${platform_assets}/${image_name}"
        if [[ ! -s "${staged_image}" ]]; then
            echo "missing staged documentation image: ${staged_image}" >&2
            exit 1
        fi
        if ! cmp -s "${source_image}" "${staged_image}"; then
            echo "stale staged documentation image: ${staged_image}" >&2
            exit 1
        fi
    done
done

echo "Full Core, ESP, and RP documentation snapshot verified."
