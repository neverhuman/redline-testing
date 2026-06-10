#!/usr/bin/env bash
#
# Cost-budget lane: validates the zero-spend manifest and emits a receipt.
# redline-testing has no paid or unbounded runtime surface; this lane proves
# the budgets, quota caps, and stop conditions stay at zero.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
. "$repo_root/ops/ci/lib.sh"

cd "$repo_root"
mkdir -p target/jankurai

log "cost-budget: validating zero-spend manifest"
python3 - <<'PY'
import json
import tomllib
from pathlib import Path

manifest_path = Path("agent/cost-budget.toml")
manifest = tomllib.loads(manifest_path.read_text())

required = {"default_external_spend_usd": 0, "default_network_spend_usd": 0}
for key, expected in required.items():
    if manifest.get(key) != expected:
        raise SystemExit(f"{key} must be {expected}")

quota_caps = manifest.get("quota_caps", {})
for key in ("external_api_usd", "telegram_paid_usd", "model_api_usd"):
    if quota_caps.get(key) != 0:
        raise SystemExit(f"quota cap {key} must be zero by default")

stop_conditions = manifest.get("stop_conditions", {})
for key in ("on_missing_receipt", "on_unknown_paid_tool", "on_quota_exceeded", "on_kill_switch"):
    if stop_conditions.get(key) is not True:
        raise SystemExit(f"stop condition {key} must be true")

receipt = {
    "ok": True,
    "manifest": str(manifest_path),
    "default_external_spend_usd": manifest["default_external_spend_usd"],
    "quota_caps": quota_caps,
    "kill_switch_env": manifest["kill_switch_env"],
    "stop_conditions": stop_conditions,
}
Path("target/jankurai/cost-budget.json").write_text(json.dumps(receipt, indent=2) + "\n")
PY

log "cost-budget: complete"
