#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/.." && pwd)
evaluator="$repo_dir/target/debug/examples/evaluate"

(cd -- "$repo_dir" && cargo build --quiet --example evaluate)

status=0
while IFS= read -r example; do
    if output=$("$evaluator" "$example" 'tags.isAwesome=true' 2>&1); then
        :
    else
        printf 'failed: %s\n%s\n' "$example" "$output"
        status=1
    fi
done < <(sed -nE 's/^- `([^`]*)`$/\1/p' "$repo_dir/README.md")

exit "$status"
