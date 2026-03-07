# Build RP docs and open them in a browser
show-docs-rp:
    cd crates/device-envoy-rp && just show-docs-rp

# Build ESP docs and open them in a browser
show-docs-esp:
    cd crates/device-envoy-esp && just show-docs-esp

# Run all checks across all three crates.
check-all:
    cd crates/device-envoy-core && cargo test --features host
    cd crates/device-envoy-core && cargo check --features host --examples
    cd crates/device-envoy-esp && cargo check-all
    cd crates/device-envoy-rp && cargo check-all
