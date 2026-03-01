# Run all checks across all three crates.
check-all:
    cd crates/device-envoy-core && cargo test
    cd crates/device-envoy-esp && cargo check --features host --release
    cd crates/device-envoy-rp && cargo check-all
