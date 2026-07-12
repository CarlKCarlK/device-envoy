# Build the versioned DNS tester WASM package.
build-dns-tester version="v1":
    mkdir -p "docs/dns-tester/{{version}}/pkg"
    cargo build -p device-envoy-dns-tester-wasm --release --target wasm32-unknown-unknown
    wasm-bindgen target/wasm32-unknown-unknown/release/device_envoy_dns_tester_wasm.wasm --out-dir "docs/dns-tester/{{version}}/pkg" --target web

# Build and serve the DNS tester Pages tree for browser review.
run-dns-tester version="v1" port="8000":
    just build-dns-tester "{{version}}"
    python3 -m http.server "{{port}}" --bind 127.0.0.1 --directory "docs/dns-tester/{{version}}"

# Build the versioned Conway WASM package.
build-conway version="v2":
    mkdir -p "docs/conway/{{version}}/pkg"
    cargo build -p device-envoy-conway-wasm --release --target wasm32-unknown-unknown
    wasm-bindgen target/wasm32-unknown-unknown/release/device_envoy_conway_wasm.wasm --out-dir "docs/conway/{{version}}/pkg" --target web

# Build and serve the Conway Pages tree for browser review.
run-conway version="v2" port="8000":
    just build-conway "{{version}}"
    python3 -m http.server "{{port}}" --bind 127.0.0.1 --directory "docs/conway/{{version}}"

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
    cargo doc -p device-envoy-core --no-deps --features host,wasm,doc-images

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
