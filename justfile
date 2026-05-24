set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default recipe runs the same lane CI runs.
default: pr-ci

# Fast format + compile check (matches the start of pr-ci, no test run).
check:
    cargo fmt --check
    cargo check --locked --all-targets

# Run the Rust test suite (28 integration + unit tests).
test:
    cargo test --locked

# Full verification mirror of GitHub Actions ci.yml.
verify: pr-ci

pr-ci:
    scripts/ci-local.sh pr-ci

# Build + package the release tarball + manifest. Refactored to invoke
# scripts/release-package.sh so the artifact_hashes set is glob-driven (every
# file under dist/<package>/ is enumerated, not hand-listed). New corpora /
# schemas / templates auto-appear without editing this recipe.
release-local:
    scripts/release-package.sh

# Run the corpus generator's drift check (re-emits matrix-product shards and
# fails on diff). Wired into pr-ci via xtask alongside the existing checks.
generate-check:
    cargo run -p xtask --release -- generate --check

# Validate every shard under corpus/sqlite_parity/cases/ against sqlite3.
ship-gate:
    cargo run -p xtask --release -- ship-gate
