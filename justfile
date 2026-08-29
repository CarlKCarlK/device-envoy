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

# Build the RP-only docs preview and open it in a browser
show-docs-rp:
    cd crates/device-envoy-rp && just show-docs-rp

# Build the authoritative documentation snapshot and open all three indexes
show-docs:
    just docs
    bash -lc 'if command -v xdg-open >/dev/null; then xdg-open target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; elif command -v wslview >/dev/null; then wslview target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; else echo "Core docs built at target/doc/device_envoy_core/index.html"; fi'
    crates/device-envoy-esp/scripts/open-docs-esp.sh
    crates/device-envoy-rp/scripts/open-docs-rp.sh

# Build the Core-only docs preview and open it in a browser
show-docs-core:
    just docs-core-only
    bash -lc 'if command -v xdg-open >/dev/null; then xdg-open target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; elif command -v wslview >/dev/null; then wslview target/doc/device_envoy_core/index.html >/dev/null 2>&1 || true; else echo "Docs built at target/doc/device_envoy_core/index.html"; fi'

# Build the ESP-only docs preview and open it in a browser
show-docs-esp:
    cd crates/device-envoy-examples-esp && just show-docs-esp

# Build only Core docs. This invalidates the authoritative full snapshot.
docs-core-only:
    cargo doc -p device-envoy-core --no-deps --features host,wasm,doc-images

# Build only ESP docs. This invalidates the authoritative full snapshot.
docs-esp-only:
    cd crates/device-envoy-examples-esp && just docs-esp-only

# Build only RP docs. This invalidates the authoritative full snapshot.
docs-rp-only:
    cd crates/device-envoy-rp && just docs-rp-only

# Build and verify the one authoritative, reviewable documentation snapshot.
docs:
    just docs-rp-only
    just docs-esp-only
    just docs-core-only
    bash scripts/check-docs-full.sh

# Rebuild authoritative docs and bundle CYD-related pages as agent-readable Markdown.
docs-agent-text: docs
    python3 scripts/rustdoc_sites_to_markdown.py --output target/device-envoy-cyd-rustdoc.md

# Build an incomplete ESP-only preview without regenerating assets or validating output
docs-esp-only-unvalidated:
    cd crates/device-envoy-examples-esp && just docs-esp-only-unvalidated

# Build an incomplete RP-only preview without images or validation
docs-rp-only-no-images:
    cd crates/device-envoy-rp && just docs-rp-only-no-images

# Build incomplete RP and ESP docs previews; never use this output for review or publishing
docs-incomplete:
    @echo "WARNING: building incomplete documentation previews; use 'just docs' for authoritative output." >&2
    just docs-rp-only-no-images
    just docs-esp-only-unvalidated
