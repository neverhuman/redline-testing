@/home/ubuntu/.codex/RTK.md

# redline-testing Agent Router

Mission: keep the external RedlineDB conformance runner deterministic,
release-packaged, and compatible with RedlineDB's pinned artifact consumer.

Start here:
- `.jankurai/owner-map.json`
- `.jankurai/test-map.json`
- `.jankurai/proof-lanes.toml`
- `.jankurai/generated-zones.toml`
- `.jankurai/unsafe-ledger.toml`

Rules:
- Keep edits scoped to this repository.
- Never hand-edit paths listed in `.jankurai/generated-zones.toml`.
- Preserve JSONL field compatibility for RedlineDB report parsing.
- Treat `rtk cargo fmt --check`, `rtk cargo check --locked`,
  `rtk cargo test --locked`, and `rtk just release-local` as the local proof.
