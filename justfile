# Run all checks across all three crates.
check-all:
    cd crates/device-envoy-core && cargo test --features host
    cd crates/device-envoy-esp && cargo check-all
    cd crates/device-envoy-rp && cargo check-all
