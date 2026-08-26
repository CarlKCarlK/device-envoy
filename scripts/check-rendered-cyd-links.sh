#!/usr/bin/env bash
set -euo pipefail

# Rustdoc leaves unresolved intra-doc links as literal `[<code>…</code>]` text.
# Keep this rendered-output check alongside the documentation update scripts so
# a warning-free rustdoc build cannot hide broken CYD navigation.
for cyd_docs in "$@"; do
    if [[ ! -d "$cyd_docs" ]]; then
        echo "missing rendered CYD documentation directory: $cyd_docs" >&2
        exit 1
    fi

    set +e
    rg -n '\[<code>[^<]+</code>\]' "$cyd_docs"
    scan_status=$?
    set -e
    case "$scan_status" in
        1) ;;
        0)
            echo "unresolved rendered CYD documentation link in $cyd_docs" >&2
            exit 1
            ;;
        *)
            echo "failed to scan rendered CYD documentation in $cyd_docs (rg exit $scan_status)" >&2
            exit "$scan_status"
            ;;
    esac
done
