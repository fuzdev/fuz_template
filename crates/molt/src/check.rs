use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::anchors;
use crate::config::MoltConfig;
use crate::error::CliError;
use crate::features;
use crate::plan::{build_plan, conflicts, verify};
use crate::templates;

/// What `check_all` found, split by remedy.
#[derive(Debug, Default)]
pub struct CheckIssues {
    /// Anchors or embedded templates the template drifted from.
    pub drift: Vec<String>,
    /// Template files in the way of files molt creates.
    pub conflicts: Vec<String>,
}

impl CheckIssues {
    pub const fn is_empty(&self) -> bool {
        self.drift.is_empty() && self.conflicts.is_empty()
    }
}

/// Runs `molt check`: verifies every anchor and embedded-template invariant
/// against the tree at `root`.
pub fn run(root: &Path) -> Result<ExitCode, CliError> {
    let issues = check_all(root)?;
    if issues.is_empty() {
        println!("molt check passed: all anchors and embedded templates match");
        return Ok(ExitCode::SUCCESS);
    }
    if !issues.drift.is_empty() {
        eprintln!(
            "molt check failed \u{2014} the template drifted from molt's anchors or embedded templates:"
        );
        for issue in &issues.drift {
            eprintln!("  {issue}");
        }
        eprintln!(
            "(update the anchors in crates/molt/src/anchors.rs and src/lib/molt.ts, and the shared templates in crates/molt/templates/, in the same change)"
        );
    }
    if !issues.conflicts.is_empty() {
        eprintln!(
            "molt check failed \u{2014} template files are in the way of files molt creates:"
        );
        for issue in &issues.conflicts {
            eprintln!("  {issue}");
        }
        eprintln!("(molt creates these on eject \u{2014} remove or rename them in the template)");
    }
    // both are caller-must-fix, same dialect as `CliError::Drift`
    Ok(ExitCode::from(2))
}

/// Verifies the plans for both sample configs, covering every anchor molt can
/// touch (each feature exercised kept in one config and stripped in the other),
/// plus the embedded-template invariants that anchors alone can't see.
pub fn check_all(root: &Path) -> Result<CheckIssues, CliError> {
    let mut issues = Vec::new();
    let mut conflict_issues = Vec::new();
    for config in sample_configs() {
        let plan = build_plan(&config);
        issues.extend(verify(root, &plan)?);
        conflict_issues.extend(conflicts(root, &plan));
    }
    // the embedded workspace template must stay byte-identical to the live
    // root Cargo.toml apart from the members and license lines — otherwise
    // an edit to the live lints/profile/deps would silently ship a stale
    // workspace to every molted project while the members anchor still matched
    let live_path = root.join("Cargo.toml");
    let live = fs::read_to_string(&live_path).map_err(|source| CliError::Io {
        path: live_path,
        source,
    })?;
    let rendered = templates::render(
        templates::WORKSPACE_CARGO_TOML,
        &[
            ("members = [__MEMBERS__]", anchors::WORKSPACE_MEMBERS),
            ("__LICENSE__", anchors::WORKSPACE_LICENSE),
        ],
    );
    if live != rendered {
        issues.push(
            "Cargo.toml: drifted from crates/molt/templates/workspace_cargo.toml.in (only the members and license lines may differ)"
                .to_owned(),
        );
    }
    issues.sort();
    issues.dedup();
    conflict_issues.sort();
    conflict_issues.dedup();
    Ok(CheckIssues {
        drift: issues,
        conflicts: conflict_issues,
    })
}

/// Two configs that together exercise every plan branch: one keeps every
/// registry feature (derived from `features::FEATURES`, so a new feature is
/// covered without touching this) and sets every optional value, one strips
/// every feature and clears the optional values.
pub fn sample_configs() -> [MoltConfig; 2] {
    [
        MoltConfig {
            name: "sample_app".to_owned(),
            npm_name: "@sample/sample_app".to_owned(),
            // contains "app_cli" to prove the crate rename can't corrupt it
            description: "a sample app that replaces app_cli".to_owned(),
            domain: Some("sample.example.com".to_owned()),
            repo_url: Some("https://github.com/sample/sample_app".to_owned()),
            kept: features::FEATURES.iter().map(|f| f.id).collect(),
        },
        MoltConfig {
            name: "plain_app".to_owned(),
            npm_name: "plain_app".to_owned(),
            description: String::new(),
            domain: None,
            repo_url: None,
            kept: BTreeSet::new(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn anchors_match_the_template() {
        let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let issues = check_all(&root).unwrap();
        assert!(
            issues.is_empty(),
            "template drifted from molt's anchors:\n{}",
            [issues.drift, issues.conflicts].concat().join("\n")
        );
    }
}
