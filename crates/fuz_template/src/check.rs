use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use crate::config::MoltConfig;
use crate::error::CliError;
use crate::features;
use crate::plan::{build_plan, verify};

/// Runs `molt check`: verifies every anchor against the tree at `root`.
pub fn run(root: &Path) -> Result<ExitCode, CliError> {
    let issues = check_all(root)?;
    if issues.is_empty() {
        println!("molt check passed: all anchors match the template");
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("molt check failed — the template drifted from molt's anchors:");
        for issue in &issues {
            eprintln!("  {issue}");
        }
        eprintln!("(update crates/fuz_template/src/anchors.rs in the same change)");
        Ok(ExitCode::FAILURE)
    }
}

/// Verifies the plans for both sample configs, covering every anchor molt can
/// touch (each feature exercised kept in one config and stripped in the other).
pub fn check_all(root: &Path) -> Result<Vec<String>, CliError> {
    let mut issues = Vec::new();
    for config in sample_configs() {
        issues.extend(verify(root, &build_plan(&config))?);
    }
    issues.sort();
    issues.dedup();
    Ok(issues)
}

/// Two configs that together exercise every plan branch: one keeps
/// rust/cli/docs and sets every optional value, one strips them and clears
/// the optional values (while keeping the github extras).
pub fn sample_configs() -> [MoltConfig; 2] {
    [
        MoltConfig {
            name: "sample_app".to_owned(),
            npm_name: "@sample/sample_app".to_owned(),
            description: "a sample app".to_owned(),
            domain: Some("sample.example.com".to_owned()),
            repo_url: Some("https://github.com/sample/sample_app".to_owned()),
            kept: BTreeSet::from([features::RUST, features::CLI, features::DOCS]),
        },
        MoltConfig {
            name: "plain_app".to_owned(),
            npm_name: "plain_app".to_owned(),
            description: String::new(),
            domain: None,
            repo_url: None,
            kept: BTreeSet::from([features::GITHUB_EXTRAS]),
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
            issues.join("\n")
        );
    }
}
