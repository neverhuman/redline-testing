#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [ "${CI:-}" = "true" ]; then
    # Keep CI cargo state local to the checkout so shared runner caches do not
    # become a serialization point for unrelated pipelines.
    export CARGO_HOME="$repo_root/.cargo"
    export CARGO_TARGET_DIR="$repo_root/target"
fi

ci_run() {
    if command -v rtk >/dev/null 2>&1; then
        rtk "$@"
    else
        "$@"
    fi
}
