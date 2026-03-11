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
    cargo fmt --all
    bash -lc 'set -euo pipefail; \
      failures=0; \
      (cd crates/device-envoy-core && cargo test --features host && cargo check --features host --examples) & core_pid=$!; \
      (cd crates/device-envoy-esp && cargo check-all) & esp_pid=$!; \
      (cd crates/device-envoy-rp && cargo check-all) & rp_pid=$!; \
      for check_pid in "$core_pid" "$esp_pid" "$rp_pid"; do \
        if ! wait "$check_pid"; then failures=1; fi; \
      done; \
      exit "$failures"'
