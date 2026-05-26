#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'ci doctor: %s\n' "$1" >&2
    exit 1
}

grep -q 'merge_request_event' "$repo_root/.gitlab-ci.yml" || fail "gitlab ci must support merge requests"
grep -F -q '$CI_COMMIT_TAG' "$repo_root/.gitlab-ci.yml" || fail "gitlab ci must support release tags"
grep -q 'scripts/ci-local.sh pr-ci' "$repo_root/.gitlab-ci.yml" || fail "gitlab ci must run the pr-ci lane"
grep -q 'scripts/ci-local.sh release' "$repo_root/.gitlab-ci.yml" || fail "gitlab ci must run the release lane"
grep -q 'beyond_postgres:' "$repo_root/.gitlab-ci.yml" || fail "gitlab ci must keep the beyond-postgres lane"
grep -q 'workflow_dispatch' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must support workflow_dispatch"
grep -q 'merge_group' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must support merge_group"
grep -q 'scripts/ci-local.sh pr-ci' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must run the pr-ci lane"
grep -q 'beyond-postgres:' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must keep the beyond-postgres lane"
grep -q 'python3 scripts/update-badge.py' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must keep the badge update lane"
grep -q 'actions/upload-artifact' "$repo_root/.github/workflows/ci.yml" || fail "github ci workflow must upload the beyond-SQLite artifact"
grep -q 'workflow_dispatch' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must support workflow_dispatch"
grep -q 'tags:' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must trigger on tags"
grep -q 'scripts/ci-local.sh pr-ci' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must run the pr-ci lane"
grep -q 'scripts/ci-local.sh release' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must run the release lane"
grep -q 'actions/attest-build-provenance' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must attest release artifacts"
grep -q 'gh release create' "$repo_root/.github/workflows/release.yml" || fail "github release workflow must publish a GitHub release"
grep -q 'ops/ci/pr-ci.sh' "$repo_root/scripts/ci-local.sh" || fail "ci-local must dispatch to ops/ci/pr-ci.sh"
grep -q 'ops/ci/release.sh' "$repo_root/scripts/ci-local.sh" || fail "ci-local must dispatch to ops/ci/release.sh"
