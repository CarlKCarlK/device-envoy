# Build the versioned DNS tester WASM package.
build-dns-tester version="v1":
    mkdir -p "docs/dns-tester/{{version}}/pkg"
    cp crates/device-envoy-core/www/cyd-simulator.js "docs/dns-tester/{{version}}/cyd-simulator.js"
    cp crates/device-envoy-core/www/cyd-simulator.css "docs/dns-tester/{{version}}/cyd-simulator.css"
    cp crates/device-envoy-core/www/case.png "docs/dns-tester/{{version}}/case.png"
    cp crates/device-envoy-core/www/desk.jpg "docs/dns-tester/{{version}}/desk.jpg"
    cargo build -p device-envoy-dns-tester-wasm --release --target wasm32-unknown-unknown
    wasm-bindgen target/wasm32-unknown-unknown/release/device_envoy_dns_tester_wasm.wasm --out-dir "docs/dns-tester/{{version}}/pkg" --target web
    wasm_version=$(sha256sum target/wasm32-unknown-unknown/release/device_envoy_dns_tester_wasm.wasm | cut -c1-12); sed -E -i "s/\\?v=[A-Za-z0-9_-]+/\\?v=$wasm_version/g" "docs/dns-tester/{{version}}/main.js" "docs/dns-tester/{{version}}/index.html"

# Run DNS tester browser-boundary tests. Requires wasm-pack and Chromium/Chrome.
test-dns-tester-browser:
    wasm-pack test --headless --chrome crates/device-envoy-dns-tester-wasm

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

# Build all public documentation and open the Core, ESP, and RP indexes
show-docs:
    just update-docs
    bash -lc 'if command -v xdg-open >/dev/null; then xdg-open target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; elif command -v wslview >/dev/null; then wslview target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; else echo "Core docs built at target/doc/device_envoy_core/index.html"; fi'
    crates/device-envoy-esp/scripts/open-docs-esp.sh
    crates/device-envoy-rp/scripts/open-docs-rp.sh

# Build core docs and open them in a browser
show-docs-core:
    just update-docs-core
    bash -lc 'if command -v xdg-open >/dev/null; then xdg-open target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; elif command -v wslview >/dev/null; then wslview target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; else echo "Docs built at target/doc/device_envoy_core/index.html"; fi'

# Build ESP docs and open them in a browser
show-docs-esp:
    cd crates/device-envoy-examples-esp && just show-docs-esp

# Update core docs only
update-docs-core:
    cargo doc -p device-envoy-core --no-deps --features host,wasm,doc-images

# Update ESP docs only
update-docs-esp:
    cd crates/device-envoy-examples-esp && just update-docs-esp

# Update RP docs only
update-docs-rp:
    cd crates/device-envoy-rp && just update-docs-rp

# Update Core, ESP, and RP documentation
update-docs:
    just update-docs-rp
    just update-docs-esp
    just update-docs-core

# Update ESP docs only (fast path)
update-docs-esp-fast:
    cd crates/device-envoy-examples-esp && just update-docs-esp-fast

# Update RP docs only (fast path)
update-docs-rp-fast:
    cd crates/device-envoy-rp && just update-docs-rp-fast

# Update RP + ESP docs (fast path)
update-docs-fast:
    just update-docs-rp-fast
    just update-docs-esp-fast
