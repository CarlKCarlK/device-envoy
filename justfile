# Build RP docs and open them in a browser
show-docs-rp:
    cd crates/device-envoy-rp && just show-docs-rp

# Build ESP docs and open them in a browser
show-docs-esp:
    cd crates/device-envoy-esp && just show-docs-esp

# Update ESP docs only
update-docs-esp:
    cd crates/device-envoy-esp && just update-docs-esp

# Run all checks across all three crates.
check-all:
    cargo run --manifest-path xtask/Cargo.toml -- check-all
