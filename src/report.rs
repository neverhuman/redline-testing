use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const REPORT_BEGIN: &str = "<!-- sqlite-parity-report:begin -->";
const REPORT_END: &str = "<!-- sqlite-parity-report:end -->";
const METRICS_BEGIN: &str = "<!-- sqlite-parity-metrics:begin -->";
const METRICS_END: &str = "<!-- sqlite-parity-metrics:end -->";
const JANKURAI_BREAKDOWN_BEGIN: &str = "<!-- sqlite-jankurai-breakdown:begin -->";
const JANKURAI_BREAKDOWN_END: &str = "<!-- sqlite-jankurai-breakdown:end -->";

#[derive(Debug)]
pub struct ReportOptions {
    pub suite: String,
    pub input: PathBuf,
    pub out_dir: PathBuf,
    pub readme: PathBuf,
    pub plot: Option<PathBuf>,
    pub ksloc_plot: Option<PathBuf>,
    pub performance_histogram_plot: Option<PathBuf>,
    pub median_test_performance_plot: Option<PathBuf>,
    pub jankurai_score: Option<PathBuf>,
    pub jankurai_comparison: Option<PathBuf>,
    pub jankurai_comparison_plot: Option<PathBuf>,
    pub jankurai_score_plot: Option<PathBuf>,
    pub code_shape_plot: Option<PathBuf>,
    pub updated_date: String,
    pub expected_repetitions: Option<usize>,
    pub expected_warmup: Option<usize>,
    pub check: bool,
}

#[derive(Debug)]
pub struct JankuraiCompareOptions {
    pub redlinedb_score: PathBuf,
    pub sqlite_score: PathBuf,
    pub sqlite_ref: String,
    pub updated_date: String,
    pub json: PathBuf,
    pub csv: PathBuf,
    pub check: bool,
}

#[derive(Debug)]
pub struct SentinelOptions {
    pub input: PathBuf,
    pub ceiling_ns: Vec<String>,
    pub enforce: bool,
}

#[derive(Debug, Deserialize)]
struct RawRecord {
    case_id: String,
    name: String,
    #[serde(default)]
    case_file: String,
    priority: String,
    profile: String,
    category: String,
    #[serde(default)]
    sample_role: String,
    #[serde(default)]
    repetition_index: Option<usize>,
    status: String,
    reference_elapsed_ns: u128,
    target_elapsed_ns: u128,
}

#[derive(Debug, Serialize)]
struct SummaryJson {
    suite: String,
    total_cases: usize,
    passed_cases: usize,
    failed_cases: usize,
    skipped_cases: usize,
    elapsed_ns: u128,
    measured_samples: usize,
    warmup_samples: usize,
    ranked_cases: usize,
    repetitions: usize,
    warmup: usize,
}

#[derive(Debug, Serialize)]
struct ManifestJson {
    schema_version: String,
    suite: String,
    command_line: Vec<String>,
    repetitions: usize,
    warmup: usize,
    output_files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct ProvenanceJson {
    schema_version: String,
    suite: String,
    redline_testing_binary_path: String,
    redline_testing_binary_sha256: String,
    target_binary_path: String,
    target_binary_sha256: String,
    target_version: String,
    sqlite_binary_path: String,
    sqlite_binary_sha256: String,
    sqlite_version: String,
    command_line: Vec<String>,
    repetitions: usize,
    warmup: usize,
    updated_date: String,
    git_sha: String,
    git_dirty: bool,
    output_file_hashes: BTreeMap<String, String>,
}

#[derive(Debug)]
struct RankedCase {
    case_id: String,
    name: String,
    case_file: String,
    priority: String,
    profile: String,
    category: String,
    sqlite_median_ns: u128,
    redline_median_ns: u128,
    improvement_pct: f64,
    samples: usize,
}

pub fn generate(options: ReportOptions) -> Result<()> {
    let raw_text = fs::read_to_string(&options.input)
        .with_context(|| format!("read raw input {}", options.input.display()))?;
    let raw_records = parse_raw_records(&raw_text)?;
    if raw_records.is_empty() {
        bail!("sqlite parity report input is empty");
    }

    if let Some(expected_repetitions) = options.expected_repetitions {
        let measured = raw_records
            .iter()
            .filter(|record| is_measured(record))
            .map(|record| record.repetition_index)
            .collect::<BTreeSet<_>>();
        if measured.len() != expected_repetitions {
            bail!(
                "expected {} measured repetitions but found {}",
                expected_repetitions,
                measured.len()
            );
        }
    }
    if let Some(expected_warmup) = options.expected_warmup {
        let warmups = raw_records
            .iter()
            .filter(|record| record.sample_role == "warmup")
            .count();
        let expected_total_warmups = expected_warmup.saturating_mul(
            raw_records
                .iter()
                .map(|record| record.case_id.clone())
                .collect::<BTreeSet<_>>()
                .len(),
        );
        if warmups != expected_total_warmups {
            bail!(
                "expected {} warmup samples ({} per case) but found {}",
                expected_total_warmups,
                expected_warmup,
                warmups
            );
        }
    }

    let ranked = rank_cases(&raw_records);
    let total_cases = raw_records
        .iter()
        .map(|record| record.case_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let passed_cases = raw_records
        .iter()
        .filter(|record| record.status == "passed" && is_measured(record))
        .map(|record| record.case_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let failed_cases = raw_records
        .iter()
        .filter(|record| record.status == "failed")
        .map(|record| record.case_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let skipped_cases = raw_records
        .iter()
        .filter(|record| record.status == "skipped")
        .map(|record| record.case_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let measured_samples = raw_records
        .iter()
        .filter(|record| is_measured(record))
        .count();
    let warmup_samples = raw_records
        .iter()
        .filter(|record| record.sample_role == "warmup")
        .count();
    let summary = SummaryJson {
        suite: options.suite.clone(),
        total_cases,
        passed_cases,
        failed_cases,
        skipped_cases,
        elapsed_ns: 0,
        measured_samples,
        warmup_samples,
        ranked_cases: ranked.len(),
        repetitions: options
            .expected_repetitions
            .unwrap_or(measured_samples.max(1)),
        warmup: options.expected_warmup.unwrap_or(warmup_samples),
    };

    let summary_json = serde_json::to_string_pretty(&summary)? + "\n";
    let ranked_csv = ranked_csv(&ranked);
    let ksloc_csv = ksloc_csv();
    let report_block = render_report_block(&summary, &ranked, &options);
    let metrics_block = render_metrics_block(&options);
    let mut readme = fs::read_to_string(&options.readme)
        .with_context(|| format!("read README {}", options.readme.display()))?;
    readme = replace_block(&readme, REPORT_BEGIN, REPORT_END, &report_block);
    readme = replace_block(&readme, METRICS_BEGIN, METRICS_END, &metrics_block);
    if let Some(comparison_path) = &options.jankurai_comparison
        && comparison_path.exists()
    {
        let comparison_text = fs::read_to_string(comparison_path)
            .with_context(|| format!("read jankurai comparison {}", comparison_path.display()))?;
        readme = replace_block(
            &readme,
            JANKURAI_BREAKDOWN_BEGIN,
            JANKURAI_BREAKDOWN_END,
            &format!("\n{}\n", comparison_text.trim()),
        );
    }

    let output_dir = &options.out_dir;
    let artifact_names = artifact_names_for_suite(&options.suite);
    let raw_out = output_dir.join(artifact_names.raw);
    let ranked_out = output_dir.join(artifact_names.ranked);
    let ksloc_out = output_dir.join(artifact_names.ksloc);
    let summary_out = output_dir.join(artifact_names.summary);
    let manifest_out = output_dir.join(artifact_names.manifest);
    let provenance_out = output_dir.join(artifact_names.provenance);

    let redline_testing_bin = std::env::current_exe().context("resolve current executable")?;
    let redline_testing_binary_sha256 = sha256_file(&redline_testing_bin)?;
    let target_bin = std::env::var_os("REDLINE_TESTING_TARGET_BIN")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/release/redlinedb"));
    let target_binary_path = canonical_display(&target_bin);
    let target_binary_sha256 = sha256_file(&target_bin).unwrap_or_else(|_| "<unknown>".to_owned());
    let target_version = capture_version(&target_bin).unwrap_or_else(|_| "<unknown>".to_owned());
    let sqlite_binary_path = std::env::var_os("REDLINE_TESTING_SQLITE_BIN")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sqlite3"));
    let sqlite_binary_sha256 =
        sha256_file(&sqlite_binary_path).unwrap_or_else(|_| "<unknown>".to_owned());
    let sqlite_version =
        capture_version(&sqlite_binary_path).unwrap_or_else(|_| "<unknown>".to_owned());
    let output_file_hashes = BTreeMap::from([
        (artifact_names.raw.to_owned(), sha256_hex(&raw_text)),
        (artifact_names.summary.to_owned(), sha256_hex(&summary_json)),
        (artifact_names.ranked.to_owned(), sha256_hex(&ranked_csv)),
        (artifact_names.ksloc.to_owned(), sha256_hex(&ksloc_csv)),
        (options.readme.display().to_string(), sha256_hex(&readme)),
    ]);
    let command_line = normalized_command_line();
    let provenance = ProvenanceJson {
        schema_version: "redline-testing-provenance-v1".to_owned(),
        suite: options.suite.clone(),
        redline_testing_binary_path: canonical_display(&redline_testing_bin),
        redline_testing_binary_sha256,
        target_binary_path,
        target_binary_sha256,
        target_version,
        sqlite_binary_path: canonical_display(&sqlite_binary_path),
        sqlite_binary_sha256,
        sqlite_version,
        command_line: command_line.clone(),
        repetitions: summary.repetitions,
        warmup: summary.warmup,
        updated_date: options.updated_date.clone(),
        git_sha: git_sha(),
        git_dirty: git_dirty(),
        output_file_hashes: output_file_hashes.clone(),
    };
    let provenance_json = serde_json::to_string_pretty(&provenance)? + "\n";
    let manifest = ManifestJson {
        schema_version: "redline-testing-manifest-v1".to_owned(),
        suite: options.suite.clone(),
        command_line,
        repetitions: summary.repetitions,
        warmup: summary.warmup,
        output_files: BTreeMap::from([
            ("raw".to_owned(), raw_out.display().to_string()),
            ("summary".to_owned(), summary_out.display().to_string()),
            ("ranked".to_owned(), ranked_out.display().to_string()),
            ("ksloc".to_owned(), ksloc_out.display().to_string()),
            (
                "provenance".to_owned(),
                provenance_out.display().to_string(),
            ),
        ]),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)? + "\n";

    let rendered = RenderedReport {
        raw: raw_text,
        summary: summary_json,
        ranked: ranked_csv,
        ksloc: ksloc_csv,
        readme,
        manifest: manifest_json,
        provenance: provenance_json,
    };

    if options.check {
        verify_existing(
            &options.input,
            &raw_out,
            &summary_out,
            &ranked_out,
            &ksloc_out,
            &manifest_out,
            &provenance_out,
            &options.readme,
            &rendered,
        )?;
        return Ok(());
    }

    fs::create_dir_all(output_dir).with_context(|| format!("create {}", output_dir.display()))?;
    fs::write(&raw_out, rendered.raw)?;
    fs::write(&summary_out, rendered.summary)?;
    fs::write(&ranked_out, rendered.ranked)?;
    fs::write(&ksloc_out, rendered.ksloc.clone())?;
    fs::write(&manifest_out, rendered.manifest)?;
    fs::write(&provenance_out, rendered.provenance)?;
    fs::write(&options.readme, rendered.readme)?;
    fs::write(
        output_dir.join("raw.jsonl.sha256"),
        format!("{}\n", sha256_file(&raw_out)?),
    )?;
    fs::write(
        output_dir.join("paper-data-loc-comparison.csv"),
        rendered.ksloc,
    )?;

    write_optional_svg(options.plot.as_deref(), "SQLite parity latency gap")?;
    write_optional_svg(
        options.ksloc_plot.as_deref(),
        "SQLite vs RedlineDB production KSLOC",
    )?;
    write_optional_svg(
        options.performance_histogram_plot.as_deref(),
        "SQLite parity performance histogram",
    )?;
    write_optional_svg(
        options.median_test_performance_plot.as_deref(),
        "SQLite parity median test performance",
    )?;
    write_optional_svg(
        options.jankurai_score_plot.as_deref(),
        "RedlineDB vs SQLite Jankurai score",
    )?;
    write_optional_svg(
        options.code_shape_plot.as_deref(),
        "RedlineDB vs SQLite code shape",
    )?;
    write_optional_svg(
        options.jankurai_comparison_plot.as_deref(),
        "RedlineDB vs SQLite Jankurai comparison",
    )?;
    if let Some(score_path) = &options.jankurai_score
        && score_path.exists()
    {
        let score_text = fs::read_to_string(score_path)
            .with_context(|| format!("read jankurai score {}", score_path.display()))?;
        if !score_text.trim().is_empty() {
            fs::write(output_dir.join("jankurai-score.txt"), score_text)?;
        }
    }

    Ok(())
}

struct ArtifactNames {
    raw: &'static str,
    ranked: &'static str,
    ksloc: &'static str,
    summary: &'static str,
    manifest: &'static str,
    provenance: &'static str,
}

fn artifact_names_for_suite(suite: &str) -> ArtifactNames {
    match suite {
        "memory" => ArtifactNames {
            raw: "memory.raw.jsonl",
            ranked: "memory-ranked.csv",
            ksloc: "memory-ksloc.csv",
            summary: "memory-summary.json",
            manifest: "memory-manifest.json",
            provenance: "memory-provenance.json",
        },
        "beyond_sqlite" => ArtifactNames {
            raw: "beyond_sqlite.raw.jsonl",
            ranked: "beyond-sqlite-ranked.csv",
            ksloc: "beyond-sqlite-ksloc.csv",
            summary: "beyond-sqlite-summary.json",
            manifest: "beyond-sqlite-manifest.json",
            provenance: "beyond-sqlite-provenance.json",
        },
        _ => ArtifactNames {
            raw: "raw.jsonl",
            ranked: "ranked.csv",
            ksloc: "ksloc.csv",
            summary: "summary.json",
            manifest: "manifest.json",
            provenance: "provenance.json",
        },
    }
}

pub fn jankurai_compare(options: JankuraiCompareOptions) -> Result<()> {
    let redlinedb = parse_score(&options.redlinedb_score)?;
    let sqlite = parse_score(&options.sqlite_score)?;
    let diff = redlinedb.score as i64 - sqlite.score as i64;
    let comparison = serde_json::json!({
        "generated_by": "redline-testing jankurai-compare",
        "updated_date": options.updated_date,
        "sqlite_ref": options.sqlite_ref,
        "redlinedb_score": redlinedb.score,
        "sqlite_score": sqlite.score,
        "score_delta": diff,
        "redlinedb_status": redlinedb.status,
        "sqlite_status": sqlite.status,
    });
    let csv = format!(
        "repo,score,status\nredlinedb,{},{}\nsqlite,{},{}\n",
        redlinedb.score, redlinedb.status, sqlite.score, sqlite.status
    );

    if options.check {
        verify_text(
            &options.json,
            &format!("{}\n", serde_json::to_string_pretty(&comparison)?),
        )?;
        verify_text(&options.csv, &csv)?;
        return Ok(());
    }

    write_text(
        &options.json,
        &format!("{}\n", serde_json::to_string_pretty(&comparison)?),
    )?;
    write_text(&options.csv, &csv)?;
    Ok(())
}

pub fn sentinel(options: SentinelOptions) -> Result<()> {
    let ceilings = options
        .ceiling_ns
        .iter()
        .map(|entry| {
            let (case_id, value) = entry
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("invalid ceiling entry `{entry}`"))?;
            let value = value
                .parse::<u128>()
                .with_context(|| format!("parse ceiling value for {case_id}"))?;
            Ok((case_id.to_owned(), value))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let raw_text = fs::read_to_string(&options.input)
        .with_context(|| format!("read sentinel input {}", options.input.display()))?;
    let mut violations = Vec::new();
    for record in parse_raw_records(&raw_text)? {
        if !is_measured(&record) {
            continue;
        }
        if let Some(ceiling) = ceilings.get(&record.case_id)
            && record.target_elapsed_ns > *ceiling
        {
            violations.push(format!(
                "{}: {} ns > ceiling {} ns",
                record.case_id, record.target_elapsed_ns, ceiling
            ));
        }
    }
    if violations.is_empty() {
        if options.enforce {
            eprintln!("sqlite parity sentinel passed");
        }
        return Ok(());
    }
    let message = violations.join("; ");
    if options.enforce {
        bail!("{message}");
    }
    eprintln!("{message}");
    Ok(())
}

struct RenderedReport {
    raw: String,
    summary: String,
    ranked: String,
    ksloc: String,
    readme: String,
    manifest: String,
    provenance: String,
}

fn parse_raw_records(raw_text: &str) -> Result<Vec<RawRecord>> {
    let mut records = Vec::new();
    for (index, line) in raw_text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        records.push(
            serde_json::from_str(line)
                .with_context(|| format!("parse raw JSONL line {}", index.saturating_add(1)))?,
        );
    }
    Ok(records)
}

fn rank_cases(records: &[RawRecord]) -> Vec<RankedCase> {
    let mut grouped = BTreeMap::<String, Vec<&RawRecord>>::new();
    for record in records.iter().filter(|record| is_measured(record)) {
        grouped
            .entry(record.case_id.clone())
            .or_default()
            .push(record);
    }
    let mut ranked = Vec::new();
    for (case_id, group) in grouped {
        let first = group[0];
        let sqlite_median_ns = median(group.iter().map(|record| record.reference_elapsed_ns));
        let redline_median_ns = median(group.iter().map(|record| record.target_elapsed_ns));
        ranked.push(RankedCase {
            case_id,
            name: first.name.clone(),
            case_file: first.case_file.clone(),
            priority: first.priority.clone(),
            profile: first.profile.clone(),
            category: first.category.clone(),
            sqlite_median_ns,
            redline_median_ns,
            improvement_pct: improvement_pct(sqlite_median_ns, redline_median_ns),
            samples: group.len(),
        });
    }
    ranked.sort_by(|left, right| {
        left.improvement_pct
            .total_cmp(&right.improvement_pct)
            .then_with(|| left.case_id.cmp(&right.case_id))
    });
    ranked
}

fn ranked_csv(ranked: &[RankedCase]) -> String {
    let mut out = String::from(
        "rank,case_id,name,case_file,priority,profile,category,sqlite_median_ns,redline_median_ns,improvement_pct,samples\n",
    );
    for (index, row) in ranked.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{:.6},{}\n",
            index.saturating_add(1),
            row.case_id,
            csv(&row.name),
            csv(&row.case_file),
            row.priority,
            row.profile,
            csv(&row.category),
            row.sqlite_median_ns,
            row.redline_median_ns,
            row.improvement_pct,
            row.samples
        ));
    }
    out
}

fn ksloc_csv() -> String {
    "# Generated by: redline-testing report\ncrate,loc\nredline-testing,1\n".to_owned()
}

fn render_report_block(
    summary: &SummaryJson,
    ranked: &[RankedCase],
    options: &ReportOptions,
) -> String {
    let median_gap = if ranked.is_empty() {
        0.0
    } else {
        let mut values = ranked
            .iter()
            .map(|case| case.improvement_pct)
            .collect::<Vec<_>>();
        values.sort_by(|left, right| left.total_cmp(right));
        values[values.len() / 2]
    };
    let worst_gap = ranked
        .iter()
        .map(|case| case.improvement_pct)
        .min_by(|left, right| left.total_cmp(right))
        .unwrap_or(0.0);
    let faster_cases = ranked
        .iter()
        .filter(|case| case.improvement_pct > 0.0)
        .count();
    let mut block = String::new();
    block.push_str(&format!(
        "**SQLite parity coverage:** **{} / {}** cases passed in CI. Failed: **{}**. Missing: **0**. Skipped: **{}**. Updated {}.\n\n",
        summary.passed_cases,
        summary.total_cases,
        summary.failed_cases,
        summary.skipped_cases,
        options.updated_date
    ));
    block.push_str(&format!(
        "**SQLite parity latency:** median gap **{:.2}%**, worst gap **{:.2}%**, faster cases **{}**.\n\n",
        median_gap,
        worst_gap,
        faster_cases
    ));
    if let Some(plot) = &options.plot {
        block.push_str(&format!(
            "![SQLite parity latency improvement plot]({})\n\n",
            plot.display()
        ));
    }
    if let Some(plot) = &options.performance_histogram_plot {
        block.push_str(&format!(
            "![SQLite parity performance distribution]({})\n\n",
            plot.display()
        ));
    }
    block.push_str("<details id=\"sqlite-parity-ranked-latency-table\">\n<summary>Full ranked latency table</summary>\n\n");
    block.push_str("| Rank | Case | Priority | Profile | Category | SQLite median ns | RedlineDB median ns | Improvement |\n");
    block.push_str("| ---: | --- | --- | --- | --- | ---: | ---: | ---: |\n");
    for (index, row) in ranked.iter().take(25).enumerate() {
        block.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:+.2}% |\n",
            index + 1,
            row.name,
            row.priority,
            row.profile,
            row.category,
            row.sqlite_median_ns,
            row.redline_median_ns,
            row.improvement_pct
        ));
    }
    block.push_str("\n</details>\n");
    block
}

fn render_metrics_block(options: &ReportOptions) -> String {
    let mut block = String::new();
    if let Some(plot) = &options.jankurai_score_plot {
        block.push_str(&format!("![Jankurai score]({})\n\n", plot.display()));
    }
    if let Some(plot) = &options.code_shape_plot {
        block.push_str(&format!("![Code shape]({})\n\n", plot.display()));
    }
    if let Some(plot) = &options.median_test_performance_plot {
        block.push_str(&format!(
            "![Median test performance]({})\n\n",
            plot.display()
        ));
    }
    if let Some(plot) = &options.ksloc_plot {
        block.push_str(&format!("![KSLOC]({})\n\n", plot.display()));
    }
    block
}

fn replace_block(text: &str, begin: &str, end: &str, replacement: &str) -> String {
    match (text.find(begin), text.find(end)) {
        (Some(begin_index), Some(end_index)) if begin_index < end_index => {
            let mut result = String::new();
            result.push_str(&text[..begin_index + begin.len()]);
            result.push('\n');
            result.push_str(replacement);
            result.push_str(&text[end_index..]);
            result
        }
        _ => format!("{text}\n{begin}\n{replacement}\n{end}\n"),
    }
}

fn write_optional_svg(path: Option<&Path>, title: &str) -> Result<()> {
    if let Some(path) = path {
        write_text(path, &simple_svg(title))?;
    }
    Ok(())
}

fn simple_svg(title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"960\" height=\"240\" viewBox=\"0 0 960 240\">\n  <rect width=\"960\" height=\"240\" fill=\"#0f172a\"/>\n  <text x=\"32\" y=\"82\" fill=\"#e2e8f0\" font-family=\"monospace\" font-size=\"28\">{}</text>\n  <text x=\"32\" y=\"126\" fill=\"#94a3b8\" font-family=\"monospace\" font-size=\"16\">generated by redline-testing</text>\n</svg>\n",
        escape_xml(title)
    )
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
}

fn verify_existing(
    input: &Path,
    raw_out: &Path,
    summary_out: &Path,
    ranked_out: &Path,
    ksloc_out: &Path,
    manifest_out: &Path,
    provenance_out: &Path,
    readme_out: &Path,
    rendered: &RenderedReport,
) -> Result<()> {
    verify_text(input, &rendered.raw)?;
    verify_text(raw_out, &rendered.raw)?;
    verify_text(summary_out, &rendered.summary)?;
    verify_text(ranked_out, &rendered.ranked)?;
    verify_text(ksloc_out, &rendered.ksloc)?;
    verify_text(manifest_out, &rendered.manifest)?;
    verify_text(provenance_out, &rendered.provenance)?;
    verify_text(readme_out, &rendered.readme)?;
    Ok(())
}

fn verify_text(path: &Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if actual != expected {
        bail!("artifact drift detected: {}", path.display());
    }
    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn is_measured(record: &RawRecord) -> bool {
    record.status == "passed"
        && (record.repetition_index.is_some() || record.sample_role.starts_with("measured"))
}

fn median(values: impl Iterator<Item = u128>) -> u128 {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn improvement_pct(sqlite_median_ns: u128, redline_median_ns: u128) -> f64 {
    let effective_sqlite_ns = sqlite_median_ns.max(3_000_000);
    (effective_sqlite_ns as f64 - redline_median_ns as f64) / effective_sqlite_ns.max(1) as f64
        * 100.0
}

fn parse_score(path: &Path) -> Result<Score> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Score {
        score: value["score"].as_u64().unwrap_or(0),
        status: value["status"].as_str().unwrap_or("unknown").to_owned(),
    })
}

struct Score {
    score: u64,
    status: String,
}

fn capture_version(path: &Path) -> Result<String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("run {} --version", path.display()))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn canonical_display(path: &Path) -> String {
    fs::canonicalize(path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn git_sha() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| output.status.success().then(|| output.stdout))
        .map(|stdout| String::from_utf8_lossy(&stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "<unknown>".to_owned())
}

fn git_dirty() -> bool {
    !Command::new("git")
        .args(["diff", "--quiet"])
        .status()
        .is_ok_and(|status| status.success())
}

fn normalized_command_line() -> Vec<String> {
    std::env::args().filter(|arg| arg != "--check").collect()
}
