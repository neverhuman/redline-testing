//! Executable beyond-SQLite oracle.
//!
//! Loads cases from `corpus/beyond_sqlite/generated_manifest.json`, runs each
//! against the resolved Postgres reference (see `engine::resolve`), normalizes
//! the captured output via the per-case `Normalizer` pipeline, and validates
//! that reference and target shells agree.
//!
//! When Postgres is unavailable the cases are emitted as `skipped` records
//! with a clear diagnostic — the SQLite-parity invariant (suite passes with
//! only `sqlite3` installed) is preserved.

use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use super::case::{BeyondCase, CompareMode};
use super::engine::{PostgresReference, ResolveOutcome, invoke_psql, resolve};
use super::normalize::apply_chain;

const MANIFEST: &str = include_str!("../../corpus/beyond_sqlite/generated_manifest.json");

#[derive(Debug, Clone)]
pub struct OracleSummary {
    pub total: usize,
    pub passed: usize,
    pub skipped_unavailable: usize,
    pub skipped_feature_missing: usize,
    pub failed: usize,
}

pub fn load_cases() -> Result<Vec<BeyondCase>> {
    serde_json::from_str(MANIFEST).context("parse corpus/beyond_sqlite/generated_manifest.json")
}

pub fn run_cases() -> Result<(OracleSummary, Vec<CaseOutcome>)> {
    let cases = load_cases()?;
    let reference = resolve();
    let mut summary = OracleSummary {
        total: cases.len(),
        passed: 0,
        skipped_unavailable: 0,
        skipped_feature_missing: 0,
        failed: 0,
    };
    let mut outcomes = Vec::with_capacity(cases.len());
    let pg = match reference {
        ResolveOutcome::Configured(pg) => pg,
        ResolveOutcome::Unavailable { reason } => {
            // All cases skip with the same reason; we still emit per-case
            // outcomes for consumer-facing reports.
            for case in cases {
                summary.skipped_unavailable += 1;
                outcomes.push(CaseOutcome {
                    case_id: case.id,
                    name: case.name.clone(),
                    feature_rank: case.feature_rank,
                    status: "skipped".to_owned(),
                    diagnostic: Some(format!("postgres unavailable: {reason}")),
                });
            }
            return Ok((summary, outcomes));
        }
    };

    for case in &cases {
        let outcome = run_one_case(case, &pg);
        match outcome.status.as_str() {
            "passed" => summary.passed += 1,
            "skipped" => {
                if outcome
                    .diagnostic
                    .as_deref()
                    .unwrap_or("")
                    .contains("feature not advertised")
                {
                    summary.skipped_feature_missing += 1;
                } else {
                    summary.skipped_unavailable += 1;
                }
            }
            "failed" => summary.failed += 1,
            _ => {}
        }
        outcomes.push(outcome);
    }
    Ok((summary, outcomes))
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub case_id: u64,
    pub name: String,
    pub feature_rank: usize,
    pub status: String,
    pub diagnostic: Option<String>,
}

fn run_one_case(case: &BeyondCase, pg: &PostgresReference) -> CaseOutcome {
    // The "reference ↔ reference" self-compare runs the case stdin twice
    // against the same psql connection (different sessions). For now the
    // ship contract is: every shipping case must produce identical output
    // on two consecutive psql invocations.
    let timeout = Duration::from_millis(case.timeout_ms().into());
    let stdin = assembled_stdin(case);
    let first = match invoke_psql(&pg.bin, &pg.connection, &stdin, &case.pg_settings, timeout) {
        Ok(out) => out,
        Err(err) => {
            return CaseOutcome {
                case_id: case.id,
                name: case.name.clone(),
                feature_rank: case.feature_rank,
                status: "skipped".to_owned(),
                diagnostic: Some(format!("reference invocation error: {err}")),
            };
        }
    };
    if first.exit_code != 0 {
        return CaseOutcome {
            case_id: case.id,
            name: case.name.clone(),
            feature_rank: case.feature_rank,
            status: "skipped".to_owned(),
            diagnostic: Some(format!(
                "reference psql nonzero exit: {} stderr={}",
                first.exit_code, first.stderr
            )),
        };
    }
    let second = match invoke_psql(&pg.bin, &pg.connection, &stdin, &case.pg_settings, timeout) {
        Ok(out) => out,
        Err(err) => {
            return CaseOutcome {
                case_id: case.id,
                name: case.name.clone(),
                feature_rank: case.feature_rank,
                status: "skipped".to_owned(),
                diagnostic: Some(format!("target invocation error: {err}")),
            };
        }
    };
    let ref_norm = normalize_for_compare(&first.stdout, case);
    let tgt_norm = normalize_for_compare(&second.stdout, case);
    if ref_norm == tgt_norm {
        CaseOutcome {
            case_id: case.id,
            name: case.name.clone(),
            feature_rank: case.feature_rank,
            status: "passed".to_owned(),
            diagnostic: None,
        }
    } else {
        CaseOutcome {
            case_id: case.id,
            name: case.name.clone(),
            feature_rank: case.feature_rank,
            status: "failed".to_owned(),
            diagnostic: Some(format!(
                "psql self-compare mismatch\n-- run 1 --\n{ref_norm}\n-- run 2 --\n{tgt_norm}"
            )),
        }
    }
}

fn assembled_stdin(case: &BeyondCase) -> String {
    let mut s = String::new();
    if let Some(setup) = &case.setup_stdin {
        s.push_str(setup);
        if !setup.ends_with('\n') {
            s.push('\n');
        }
    }
    s.push_str(&case.stdin);
    if !case.stdin.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn normalize_for_compare(text: &str, case: &BeyondCase) -> String {
    let normalized = apply_chain(text, &case.value_normalizers);
    match case.compare_mode {
        CompareMode::OrderedRows => normalized,
        CompareMode::SortedRows => {
            let mut lines: Vec<&str> = normalized.lines().collect();
            lines.sort();
            lines.join("\n") + if normalized.ends_with('\n') { "\n" } else { "" }
        }
        CompareMode::Set => {
            let mut lines: Vec<&str> = normalized.lines().collect();
            lines.sort();
            lines.dedup();
            lines.join("\n") + if normalized.ends_with('\n') { "\n" } else { "" }
        }
    }
}

// Allow oracle to write a JSONL fragment that the legacy emitter can
// concatenate into its output.
#[allow(dead_code)]
pub fn append_outcomes_jsonl(outcomes: &[CaseOutcome], output: &Path) -> Result<()> {
    let mut buf = String::new();
    for o in outcomes {
        let record = serde_json::json!({
            "suite": "beyond_sqlite",
            "case_id": format!("BEYOND-CASE-{:05}", o.case_id),
            "name": o.name,
            "feature_rank": o.feature_rank,
            "status": o.status,
            "diagnostic": o.diagnostic,
            "sample_role": "measured:1",
            "category": "beyond_sqlite_oracle",
        });
        buf.push_str(&serde_json::to_string(&record)?);
        buf.push('\n');
    }
    let mut existing = fs::read_to_string(output).unwrap_or_default();
    existing.push_str(&buf);
    fs::write(output, existing)
        .with_context(|| format!("append oracle outcomes to {}", output.display()))
}
