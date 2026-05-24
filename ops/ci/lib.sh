#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

ci_run() {
    if command -v rtk >/dev/null 2>&1; then
        rtk "$@"
    else
        "$@"
    fi
}

