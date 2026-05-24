#!/usr/bin/env bash
# Local CI dispatcher for the same proof surface used by GitHub Actions.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v rtk >/dev/null 2>&1; then
    rtk() {
        "$@"
    }
fi

usage() {
    cat >&2 <<'USAGE'
usage: scripts/ci-local.sh {pr-ci}

  pr-ci  run the exact local mirror of .github/workflows/ci.yml
USAGE
}

if [ "$#" -ne 1 ]; then
    usage
    exit 64
fi

case "$1" in
    pr-ci)
        rtk cargo fmt --check
        rtk cargo check --locked
        rtk cargo test --locked
        rtk just release-local
        ;;
    -h|--help|help)
        usage
        ;;
    *)
        printf 'ci-local: unknown lane %q\n\n' "$1" >&2
        usage
        exit 64
        ;;
esac
