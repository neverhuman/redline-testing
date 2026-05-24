#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'ci doctor: %s\n' "$1" >&2
    exit 1
}

grep -q 'workflow_dispatch' "$repo_root/.github/workflows/ci.yml" || fail "ci workflow must support workflow_dispatch"
grep -q 'merge_group' "$repo_root/.github/workflows/ci.yml" || fail "ci workflow must support merge_group"
grep -q 'workflow_dispatch' "$repo_root/.github/workflows/release.yml" || fail "release workflow must support workflow_dispatch"
grep -q 'ops/ci/pr-ci.sh' "$repo_root/scripts/ci-local.sh" || fail "ci-local must dispatch to ops/ci/pr-ci.sh"
grep -q 'ops/ci/release.sh' "$repo_root/scripts/ci-local.sh" || fail "ci-local must dispatch to ops/ci/release.sh"

