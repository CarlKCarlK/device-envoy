# Build RP docs and open them in a browser
show-docs-rp:
    cd crates/device-envoy-rp && just show-docs-rp

# Build ESP docs and open them in a browser
show-docs-esp:
    cd crates/device-envoy-esp && just show-docs-esp

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

# Run all checks across all three crates.
check-all:
    cargo run --manifest-path xtask/Cargo.toml -- check-all
