# redline-testing

Official conformance, benchmark, and report harness for RedlineDB.

This repository publishes the pinned external runner artifact consumed by
RedlineDB CI. The current implemented suites are `sqlite_parity`, `memory`,
and `all`; `all` runs every implemented suite in this release.

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

## Release

```bash
just release-local
```

This builds `target/release/redline-testing`, writes
`dist/redline-testing-0.1.1-linux-x86_64.tar.gz`, and writes the matching
`.sha256` file. The tarball contains:

```text
bin/redline-testing
release-manifest.json
corpus/sqlite_parity/generated_manifest.json
```
