# Release process

`redline-testing` ships as a single signed tarball that RedlineDB CI consumes
as a pinned artifact. The release workflow is:

1. **Version source.** `Cargo.toml` `[package].version` is the canonical
   version. `CHANGELOG.md` is the human-facing log of what changed.
2. **Local rehearsal.** Run `just release-local`. This drives
   `scripts/release-package.sh` which:
   - runs `cargo build --release --locked`
   - copies the binary + every corpus / metadata / schema / template file
     into `dist/redline-testing-<version>-linux-x86_64/`
   - SHA-256-hashes every file (except `release-manifest.json` itself and
     `bin/redline-testing`, which has its own top-level `binary_sha256`)
     via `find ... | xargs sha256sum | jq` — glob-driven so new corpus shards
     auto-appear without editing the recipe
   - writes `dist/redline-testing-<version>-linux-x86_64/release-manifest.json`
   - emits the tarball + `.sha256` sidecar in `dist/`
3. **Integrity check.** `cargo test --locked --test release_manifest_integrity`
   recomputes every bundled file's SHA-256 against the manifest. If a file is
   in the tarball but missing from `artifact_hashes` (or vice versa), this
   test fails loudly. It runs as part of `just pr-ci`.
4. **Tag the release.** `git tag -s v<version>` (signed) → `git push origin
   v<version>`. The `release` workflow at `.github/workflows/release.yml`
   triggers on tag push.
5. **CI build + attestation.** The release workflow re-runs `pr-ci`, re-builds
   the tarball via `just release-local`, requests
   [`actions/attest-build-provenance`](https://github.com/actions/attest-build-provenance)
   on the tarball + `.sha256` + `release-manifest.json`, then runs
   `gh release create v<version> --verify-tag ...`. The `--verify-tag` flag
   binds the release to the actually-pushed tag so the artifact cannot be
   accidentally attached to a mutable ref.

## Verifying a release locally

After downloading the release assets (`*.tar.gz`, `*.tar.gz.sha256`,
`release-manifest.json`):

```bash
sha256sum -c redline-testing-<version>-linux-x86_64.tar.gz.sha256
tar -xzf redline-testing-<version>-linux-x86_64.tar.gz
cd redline-testing-<version>-linux-x86_64

# Verify every artifact_hashes entry matches the bundled file.
jq -r '.artifact_hashes | to_entries[] | "\(.value)  \(.key)"' \
  release-manifest.json | sha256sum -c
```

You can also verify the Sigstore attestation:

```bash
gh attestation verify redline-testing-<version>-linux-x86_64.tar.gz \
  --repo neverhuman/redline-testing
```

## Rollback

A release tag is immutable. To "roll back" we ship a follow-up release with a
higher version that restores the prior behavior; the old tag stays in place
so downstream consumers that pinned it keep working. RedlineDB's pin lives
in `redlineDB/scripts/ci_install_redline_testing.sh` (or equivalent); ask the
RedlineDB team to bump the pin to the desired version.

## Contract

Downstream RedlineDB CI consumes:

- `bin/redline-testing` — the runner.
- `corpus/sqlite_parity/generated_manifest.json` — pinned upstream cases.
- `corpus/sqlite_parity/cases/*.json` — extended hand + generated shards.
- `corpus/beyond_sqlite/generated_manifest.json` — beyond-SQLite oracle cases.
- `metadata/beyond_sqlite/features.json` — the 12-entry rank/owner taxonomy.
- `schemas/*.json` — raw-record + release-manifest schemas.
- `templates/*.md` — report-generation README templates.

Adding files to the tarball: drop them into the appropriate source directory;
`scripts/release-package.sh` picks them up automatically and they appear in
`artifact_hashes`. The Sigstore attestation covers the tarball as a whole, so
the new file is signed in transit.
