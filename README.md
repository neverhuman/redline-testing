# redline-testing

Official conformance, benchmark, and report harness for RedlineDB.

This repository publishes the pinned external runner artifact consumed by
RedlineDB CI. The current implemented suites are `sqlite_parity`, `memory`,
`beyond_sqlite`, and `all`; `all` runs every implemented suite in this release
and writes the hash-bound `official-evidence.json` bundle consumed by RedlineDB.

## Run

```bash
redline-testing run \
  --suite sqlite_parity \
  --target-bin /path/to/redlinedb \
  --sqlite-bin /path/to/sqlite3 \
  --workers auto \
  --tmp-root auto \
  --output raw.jsonl \
  --repetitions 1 \
  --warmup 0 \
  --progress auto \
  --memory-samples
```

The runner compares the target RedlineDB shell against the SQLite CLI reference
using the migrated SQLite parity manifest in `corpus/sqlite_parity/`. Execution
is deterministic and serial in this release; `--workers auto|N` is accepted and
validated for interface stability.

`raw.jsonl` uses the RedlineDB report-parser contract, including:

```text
case_id name case_file priority profile category sample_role repetition_index
sqlite_version reference_engine target_engine reference_executable_path
target_executable_path reference_executable_sha256 target_executable_sha256
reference_version target_version status reference_elapsed_ns target_elapsed_ns
```

When `--memory-samples` is set on Linux, records may also include best-effort
RSS fields such as `memory_status`, `reference_peak_rss_kb`, and
`target_peak_rss_kb`.

`memory` is a separate official suite. It uses the same parity corpus with
Linux `/proc` sampling enabled and writes `memory.raw.jsonl`,
`memory-summary.json`, `memory-ranked.csv`, `memory-manifest.json`, and
`memory-provenance.json`. If `/proc` sampling is unavailable, correctness can
still pass and records report `memory_status: unavailable`.

`beyond_sqlite` packages the beyond-SQLite feature backlog and emits per-feature
coverage evidence. Features with promoted reference coverage are marked passed;
manifest-only backlog features are explicit skips until RedlineDB accepts an
executable contract.

`all` writes `all.jsonl`, `all-manifest.json`, every per-suite raw/summary/
ranked/manifest/provenance artifact, and `official-evidence.json`. The official
evidence JSON uses schema `redline-testing-official-evidence-v1`, records the
runner, target, SQLite reference, per-suite totals, and SHA-256 hashes for the
declared output files.

## Report

RedlineDB-facing report generation must be bound to official evidence:

```bash
redline-testing report \
  --suite sqlite_parity \
  --input raw.jsonl \
  --official-evidence official-evidence.processed.json \
  --out-dir benchmark-results/sqlite-parity/latest \
  --readme README.md \
  --updated-date 2026-05-24
```

`--official-evidence` accepts either raw `official-evidence.json` or RedlineDB's
processed `official-evidence.processed.json` and verifies that the report input
hash matches the official suite hash. Omit it only with `--local-diagnostics`
for uncommitted local diagnostics.

## Release

```bash
just release-local
```

This builds `target/release/redline-testing`, writes
`dist/redline-testing-0.1.2-linux-x86_64.tar.gz`, and writes the matching
`.sha256` file. The tarball contains:

```text
bin/redline-testing
release-manifest.json
corpus/sqlite_parity/generated_manifest.json
metadata/beyond_sqlite/features.json
schemas/*.json
templates/*.md
```

Tagged GitHub releases are built by `.github/workflows/release.yml`, publish the
tarball plus `.sha256`, and request GitHub artifact attestations for the release
assets.
