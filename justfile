# Build RP docs and open them in a browser
show-docs-rp:
    cd crates/device-envoy-rp && just show-docs-rp

# Build core docs and open them in a browser
show-docs-core:
    just update-docs-core
    bash -lc 'if command -v xdg-open >/dev/null; then xdg-open target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; elif command -v wslview >/dev/null; then wslview target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; else echo "Docs built at target/doc/device_envoy_core/index.html"; fi'

# Build ESP docs and open them in a browser
show-docs-esp:
    cd crates/device-envoy-esp && just show-docs-esp

# Update core docs only
update-docs-core:
    cargo doc -p device-envoy-core --no-deps --features host,wasm

# Update ESP docs only
update-docs-esp:
    cd crates/device-envoy-esp && just update-docs-esp

# Update RP docs only
update-docs-rp:
    cd crates/device-envoy-rp && just update-docs-rp

# Update ESP docs only (fast path)
update-docs-esp-fast:
    cd crates/device-envoy-esp && just update-docs-esp-fast

# Update RP docs only (fast path)
update-docs-rp-fast:
    cd crates/device-envoy-rp && just update-docs-rp-fast

# Update RP + ESP docs (fast path)
update-docs-fast:
    just update-docs-rp-fast
    just update-docs-esp-fast
