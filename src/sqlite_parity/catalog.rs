use anyhow::{Context, Result, bail};

use super::case::Case;

const MANIFEST: &str = include_str!("../../corpus/sqlite_parity/generated_manifest.json");

pub fn all_cases() -> Result<Vec<Case>> {
    serde_json::from_str(MANIFEST).context("parse sqlite parity generated_manifest.json")
}

pub fn selected_official_cases() -> Result<Vec<Case>> {
    let cases = all_cases()?
        .into_iter()
        .filter(|case| case.status == "active" || case.status == "catalog_only")
        .collect::<Vec<_>>();
    if cases.is_empty() {
        bail!("sqlite parity selection matched zero cases");
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite_parity::case::Priority;

    #[test]
    fn generated_catalog_has_planned_size_and_ranges() {
        let cases = all_cases().expect("sqlite parity manifest");
        assert_eq!(cases.len(), 1127);
        assert_eq!(cases[0].display_id(), "00001");
        assert_eq!(cases[1126].display_id(), "01127");
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.priority == Priority::P0)
                .count(),
            130
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.priority == Priority::P1)
                .count(),
            579
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.priority == Priority::P2)
                .count(),
            370
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.priority == Priority::P3)
                .count(),
            24
        );
    }

    #[test]
    fn official_selection_is_deterministic_subset() {
        let cases = selected_official_cases().expect("official cases");
        assert_eq!(cases.len(), 1127);
        assert_eq!(cases[0].display_id(), "00001");
        assert!(cases.windows(2).all(|pair| pair[0].id < pair[1].id));
        assert!(cases.iter().any(|case| case.priority == Priority::P4));
    }
}
