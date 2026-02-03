#!/bin/bash

set -euo pipefail

root="target/thumbv8m.main-none-eabihf/doc/device_envoy"
output="target/gathers/all_docs.txt"

test -d "$root"
mkdir -p "$(dirname "$output")"
rm -f "$output"

find "$root" -type f -name '*.html' | sort | while read -r file; do
    name="${file#$root/}"
    printf "==== FILE: %s ====\n" "$name" >> "$output"
    cat "$file" >> "$output"
    printf "\n" >> "$output"
done
