pub mod case;
mod catalog;
mod engine;
mod memory;
mod normalize;
mod report;
mod runner;
mod text;

use std::path::PathBuf;

use anyhow::Result;

pub use catalog::all_cases;
pub use runner::RunSummary;

pub struct RunConfig {
    pub reference_bin: PathBuf,
    pub target_bin: PathBuf,
    pub output: PathBuf,
    pub tmp_root: PathBuf,
    pub workers: usize,
    pub repetitions: usize,
    pub warmup: usize,
    pub progress: bool,
    pub memory_samples: bool,
}

pub fn run(config: RunConfig) -> Result<RunSummary> {
    let cases = catalog::selected_official_cases()?;
    let reference = engine::EngineSpec::new("sqlite3", config.reference_bin);
    let target = engine::EngineSpec::new("redlinedb", config.target_bin);
    runner::validate_compare_engines(&reference, &target)?;
    let capabilities = reference.sqlite_shell_capabilities()?;
    let sqlite_version = capabilities
        .as_ref()
        .map(|capabilities| capabilities.version.clone());
    let partition = engine::partition_cases(cases, capabilities.as_ref());
    runner::compare_cases(
        &partition.runnable,
        &partition.skipped,
        &reference,
        &target,
        &config.output,
        config.tmp_root,
        config.workers,
        config.warmup,
        config.repetitions,
        sqlite_version,
        config.progress,
        config.memory_samples,
    )
}
