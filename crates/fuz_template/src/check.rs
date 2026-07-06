use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::anchors;
use crate::config::MoltConfig;
use crate::error::CliError;
use crate::features;
use crate::plan::{build_plan, verify};
use crate::templates;

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
        eprintln!("(update crates/fuz_template/src/anchors.rs or templates/ in the same change)");
        Ok(ExitCode::FAILURE)
    }
}

/// Verifies the plans for both sample configs, covering every anchor molt can
/// touch (each feature exercised kept in one config and stripped in the other),
/// plus the embedded-template invariants that anchors alone can't see.
pub fn check_all(root: &Path) -> Result<Vec<String>, CliError> {
    let mut issues = Vec::new();
    for config in sample_configs() {
        issues.extend(verify(root, &build_plan(&config))?);
    }
    // the embedded workspace template must stay byte-identical to the live
    // root Cargo.toml apart from the members line — otherwise an edit to the
    // live lints/profile/deps would silently ship a stale workspace to every
    // molted project while the members anchor still matched
    let live_path = root.join("Cargo.toml");
    let live = fs::read_to_string(&live_path).map_err(|source| CliError::Io {
        path: live_path,
        source,
    })?;
    let rendered = templates::WORKSPACE_CARGO_TOML
        .replace("members = [__MEMBERS__]", anchors::WORKSPACE_MEMBERS);
    if live != rendered {
        issues.push(
            "Cargo.toml: drifted from crates/fuz_template/templates/workspace_cargo.toml.in (only the members line may differ)"
                .to_owned(),
        );
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
