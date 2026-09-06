#!/usr/bin/env bash
set -euo pipefail

mode="${1:-all}"
workspace_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

case "$mode" in
    core)
        document_paths=("target/doc/device_envoy_core/index.html")
        server_root="$workspace_root/target/doc"
        document_urls=("device_envoy_core/cyd/index.html#implementations")
        ;;
    esp)
        document_paths=("target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/index.html")
        server_root="$workspace_root/target/riscv32imac-unknown-none-elf/doc"
        document_urls=("device_envoy_esp/index.html")
        ;;
    rp)
        document_paths=("target/thumbv8m.main-none-eabihf/doc/device_envoy_rp/index.html")
        server_root="$workspace_root/target/thumbv8m.main-none-eabihf/doc"
        document_urls=("device_envoy_rp/index.html")
        ;;
    all)
        server_root="$workspace_root/target"
        document_paths=(
            "target/doc/device_envoy_core/index.html"
            "target/riscv32imac-unknown-none-elf/doc/device_envoy_esp/index.html"
            "target/thumbv8m.main-none-eabihf/doc/device_envoy_rp/index.html"
        )
        document_urls=(
            "doc/device_envoy_core/index.html"
            "riscv32imac-unknown-none-elf/doc/device_envoy_esp/index.html"
            "thumbv8m.main-none-eabihf/doc/device_envoy_rp/index.html"
        )
        ;;
    *)
        echo "Unknown documentation set '$mode'; expected core, esp, rp, or all." >&2
        exit 2
        ;;
esac

for document_path in "${document_paths[@]}"; do
    if [[ ! -f "$workspace_root/$document_path" ]]; then
        echo "Documentation not found at $workspace_root/$document_path" >&2
        echo "Build it before running this script." >&2
        exit 1
    fi
done

is_wsl=false
if [[ -r /proc/sys/kernel/osrelease ]] && grep -qi microsoft /proc/sys/kernel/osrelease; then
    is_wsl=true
fi

if $is_wsl && [[ -e /proc/sys/fs/binfmt_misc/WSLInterop ]] && command -v powershell.exe >/dev/null; then
    for document_path in "${document_paths[@]}"; do
        windows_path="$(wslpath -w "$workspace_root/$document_path")"
        powershell.exe -NoProfile -Command "Start-Process '$windows_path'" >/dev/null 2>&1
    done
    exit 0
fi

if ! $is_wsl && command -v xdg-open >/dev/null && [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
    for document_path in "${document_paths[@]}"; do
        xdg-open "$workspace_root/$document_path" >/dev/null 2>&1 &
    done
    exit 0
fi

if [[ "$(uname -s)" == "Darwin" ]] && command -v open >/dev/null; then
    for document_path in "${document_paths[@]}"; do
        open "$workspace_root/$document_path"
    done
    exit 0
fi

docs_port="${DOCS_PORT:-8000}"

landing_path="$server_root/index.html"
landing_temp="$landing_path.tmp"
if [[ "$mode" == "all" ]]; then
    printf '%s\n' \
        '<!doctype html>' \
        '<html lang="en"><head><meta charset="utf-8"><title>Device Envoy documentation</title></head>' \
        '<body><h1>Device Envoy documentation</h1><ul>' \
        '<li><a href="doc/device_envoy_core/index.html">Core</a></li>' \
        '<li><a href="riscv32imac-unknown-none-elf/doc/device_envoy_esp/index.html">ESP</a></li>' \
        '<li><a href="thumbv8m.main-none-eabihf/doc/device_envoy_rp/index.html">RP</a></li>' \
        '</ul></body></html>' > "$landing_temp"
else
    landing_target="${document_urls[0]}"
    printf '%s\n' \
        '<!doctype html>' \
        '<html lang="en"><head><meta charset="utf-8">' \
        "<meta http-equiv=\"refresh\" content=\"0; url=$landing_target\">" \
        '<title>Device Envoy documentation</title></head>' \
        "<body><p><a href=\"$landing_target\">Open Device Envoy documentation</a></p></body></html>" \
        > "$landing_temp"
fi
mv "$landing_temp" "$landing_path"

echo "A graphical browser cannot be launched from this environment."
echo "Serving the generated documentation locally. Open:"
for document_url in "${document_urls[@]}"; do
    echo "  http://127.0.0.1:$docs_port/$document_url"
done
echo "Press Ctrl+C to stop the documentation server."

if command -v miniserve >/dev/null; then
    exec miniserve --interfaces 127.0.0.1 --port "$docs_port" --index index.html "$server_root"
fi

if command -v simple-http-server >/dev/null; then
    exec simple-http-server --ip 127.0.0.1 --port "$docs_port" --index "$server_root"
fi

if command -v python3 >/dev/null; then
    exec python3 -m http.server "$docs_port" --bind 127.0.0.1 --directory "$server_root"
fi

echo "No local HTTP server was found." >&2
echo "Install the Rust-based miniserve with 'cargo install --locked miniserve'." >&2
exit 1
