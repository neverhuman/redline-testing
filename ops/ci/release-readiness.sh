#!/usr/bin/env bash
#
# Release-readiness lane: asserts the launch-gate evidence surface is present
# and emits a receipt. The harness ships a versioned tarball + manifest, so the
# gate proves security, rollback, monitoring, and provenance are documented.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
. "$repo_root/ops/ci/lib.sh"

cd "$repo_root"
mkdir -p target/jankurai

log "release-readiness: validating launch-gate evidence"
python3 - <<'PY'
import json
from pathlib import Path

required_files = [
    "CHANGELOG.md",
    "docs/release.md",
    "docs/testing.md",
    "docs/operations.md",
    "agent/cost-budget.toml",
]
missing = [p for p in required_files if not Path(p).exists()]

release = Path("docs/release.md").read_text() if Path("docs/release.md").exists() else ""
testing = Path("docs/testing.md").read_text() if Path("docs/testing.md").exists() else ""
operations = Path("docs/operations.md").read_text() if Path("docs/operations.md").exists() else ""
corpus = release + "\n" + testing + "\n" + operations

required_terms = [
    "release readiness",
    "security",
    "backups",
    "monitoring",
    "rollback",
    "abuse",
    "provenance",
    "bash ops/ci/pr-ci.sh",
    "bash ops/ci/security.sh",
    "target/jankurai",
]
missing_terms = [t for t in required_terms if t.lower() not in corpus.lower()]

receipt = {
    "ok": not missing and not missing_terms,
    "required_files": required_files,
    "missing_files": missing,
    "missing_terms": missing_terms,
    "artifact_paths": [
        "target/jankurai/release-readiness.json",
        "target/jankurai/cost-budget.json",
        "target/jankurai/security/evidence.json",
    ],
}
Path("target/jankurai/release-readiness.json").write_text(json.dumps(receipt, indent=2) + "\n")
if not receipt["ok"]:
    raise SystemExit(f"release readiness missing evidence: files={missing} terms={missing_terms}")
PY

log "release-readiness: complete"
