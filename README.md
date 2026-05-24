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

`beyond_sqlite` packages the beyond-SQLite feature backlog AND, when Postgres
is reachable, runs an executable oracle that compares `psql ↔ psql` for each
case in `corpus/beyond_sqlite/generated_manifest.json` (cases are
[normalized](src/beyond_sqlite/normalize.rs) before equality). The oracle path
is gated by `REDLINE_TESTING_POSTGRES_URL` or the conventional fallback
`/dev/shm/redline-pg-sock:5433`; when neither is available every oracle case
emits `status: skipped` with a diagnostic and the SQLite-parity suite remains
unaffected.

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

This builds `target/release/redline-testing`, runs
[`scripts/release-package.sh`](scripts/release-package.sh) — which copies every
shipped artifact under `dist/<package>/`, glob-hashes them via `find` + `jq`,
and writes the `artifact_hashes` map into `release-manifest.json` — and emits
the matching `.sha256` sidecar. The tarball contains:

```text
bin/redline-testing
release-manifest.json
corpus/sqlite_parity/generated_manifest.json    (pinned upstream)
corpus/sqlite_parity/cases/*.json               (hand-authored + xtask-generated)
corpus/beyond_sqlite/generated_manifest.json    (executable oracle cases)
metadata/beyond_sqlite/features.json
schemas/*.json
templates/*.md
```

Tagged GitHub releases are built by `.github/workflows/release.yml`, publish the
tarball plus `.sha256`, and request GitHub artifact attestations for the release
assets via `actions/attest-build-provenance` (SLSA / Sigstore).

## Corpus development

The SQLite-parity corpus is split between the pinned upstream manifest
(`corpus/sqlite_parity/generated_manifest.json`, IDs 1–1127, read-only) and
extended shards under `corpus/sqlite_parity/cases/` (IDs 10001+). Two
categories of shard:

* **Hand-authored shards** (numeric prefix without `gen_`): authored by hand,
  one shard per category. Each case captures its expected output by running
  it through `sqlite3 :memory:` once at authoring time.
* **Generator shards** (`gen_*.json`): produced by `cargo run -p xtask --
  generate` from matrix-product rule definitions in
  [`xtask/src/generators.rs`](xtask/src/generators.rs). Run
  `cargo run -p xtask -- generate --check` to detect drift between rules and
  on-disk shards.

The ship contract: **a case ships iff `sqlite3 ↔ sqlite3` self-compare
passes**. The authoritative gate is

```bash
cargo run -p xtask --release -- ship-gate
```

which walks every shard and validates each case's declared expected behavior
against a fresh `sqlite3` invocation. Failing cases must be fixed or deleted;
there is no quarantine path.

Beyond-SQLite oracle cases (`corpus/beyond_sqlite/generated_manifest.json`)
follow the same contract via `psql ↔ psql`. To run the oracle locally:

```bash
# bring up a dev-time Postgres on /dev/shm
/usr/lib/postgresql/16/bin/initdb -D /dev/shm/redline-pg-data \
  --auth-local=trust --auth-host=trust -U ubuntu
/usr/lib/postgresql/16/bin/pg_ctl -D /dev/shm/redline-pg-data \
  -l /dev/shm/redline-pg.log \
  -o "-p 5433 -k /dev/shm/redline-pg-sock -h ''" start
cargo run --release -- run --suite beyond_sqlite \
  --target-bin sqlite3 --output /tmp/beyond.jsonl
```
