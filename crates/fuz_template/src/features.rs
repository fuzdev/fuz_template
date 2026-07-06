//! The keep/strip feature registry.
//!
//! Every molt-selectable feature is one entry here: a wizard prompt, a
//! `--keep`/`--strip` id, and a plan fragment in `plan.rs` keyed off the
//! resolved set. Adding a feature means adding a registry entry, its plan
//! fragment, and covering its anchors in `check.rs`'s sample configs.

use std::collections::BTreeSet;

use crate::error::CliError;

/// A molt-selectable feature of the template.
#[derive(Debug)]
pub struct Feature {
    /// Stable id used by `--keep`/`--strip` (kebab-case).
    pub id: &'static str,
    /// Wizard prompt, phrased as "keep X?".
    pub prompt: &'static str,
    /// Whether the feature is kept when unspecified.
    pub default_keep: bool,
    /// A feature this one is part of — stripping the parent strips this too.
    pub requires: Option<&'static str>,
}

pub const RUST: &str = "rust";
pub const CLI: &str = "cli";
pub const DOCS: &str = "docs";
pub const GITHUB_EXTRAS: &str = "github-extras";

pub const FEATURES: [Feature; 4] = [
    Feature {
        id: RUST,
        prompt: "keep the Rust workspace?",
        default_keep: true,
        requires: None,
    },
    Feature {
        id: CLI,
        prompt: "keep the starter CLI crate? (renamed to crates/<name>)",
        default_keep: true,
        requires: Some(RUST),
    },
    Feature {
        id: DOCS,
        prompt: "keep the docs system? (src/routes/docs, auto-generated API docs)",
        default_keep: true,
        requires: None,
    },
    Feature {
        id: GITHUB_EXTRAS,
        prompt: "keep .github/FUNDING.yml and the issue templates?",
        default_keep: false,
        requires: None,
    },
];

/// Splits repeatable/CSV flag values into feature ids, validating each.
pub fn parse_ids(values: &[String]) -> Result<Vec<&'static str>, CliError> {
    let mut ids = Vec::new();
    for value in values {
        for raw in value.split(',') {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            let Some(feature) = FEATURES.iter().find(|f| f.id == raw) else {
                return Err(CliError::Usage(format!(
                    "unknown feature {raw:?} — valid: {}",
                    FEATURES.map(|f| f.id).join(", ")
                )));
            };
            ids.push(feature.id);
        }
    }
    Ok(ids)
}

/// Resolves the kept-feature set from `--keep`/`--strip` flags, applying
/// defaults and the `requires` cascade. `explicit` returns which features the
/// flags decided (the wizard prompts only for the rest).
pub fn resolve(
    keep: &[String],
    strip: &[String],
) -> Result<(BTreeSet<&'static str>, BTreeSet<&'static str>), CliError> {
    let keep_ids = parse_ids(keep)?;
    let strip_ids = parse_ids(strip)?;
    if let Some(id) = keep_ids.iter().find(|id| strip_ids.contains(id)) {
        return Err(CliError::Usage(format!(
            "feature {id:?} passed to both --keep and --strip"
        )));
    }
    for id in &keep_ids {
        if let Some(feature) = FEATURES.iter().find(|f| f.id == *id)
            && let Some(parent) = feature.requires
            && strip_ids.contains(&parent)
        {
            return Err(CliError::Usage(format!(
                "--keep {id} conflicts with --strip {parent} ({id} is part of {parent})"
            )));
        }
    }
    let mut kept = BTreeSet::new();
    let mut explicit = BTreeSet::new();
    for feature in &FEATURES {
        let choice = if keep_ids.contains(&feature.id) {
            explicit.insert(feature.id);
            true
        } else if strip_ids.contains(&feature.id) {
            explicit.insert(feature.id);
            false
        } else {
            feature.default_keep
        };
        if choice {
            kept.insert(feature.id);
        }
    }
    cascade(&mut kept);
    Ok((kept, explicit))
}

/// Strips any feature whose `requires` parent is stripped.
pub fn cascade(kept: &mut BTreeSet<&'static str>) {
    for feature in &FEATURES {
        if let Some(parent) = feature.requires
            && !kept.contains(parent)
        {
            kept.remove(feature.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn defaults() {
        let (kept, explicit) = resolve(&[], &[]).unwrap();
        assert_eq!(kept.into_iter().collect::<Vec<_>>(), vec![CLI, DOCS, RUST]);
        assert!(explicit.is_empty());
    }

    #[test]
    fn csv_and_repeats() {
        let (kept, _) = resolve(&strings(&["github-extras,docs"]), &strings(&["cli"])).unwrap();
        assert!(kept.contains(GITHUB_EXTRAS));
        assert!(kept.contains(DOCS));
        assert!(kept.contains(RUST));
        assert!(!kept.contains(CLI));
    }

    #[test]
    fn strip_rust_cascades_to_cli() {
        let (kept, _) = resolve(&[], &strings(&["rust"])).unwrap();
        assert!(!kept.contains(RUST));
        assert!(!kept.contains(CLI));
    }

    #[test]
    fn conflicts_error() {
        assert!(resolve(&strings(&["rust"]), &strings(&["rust"])).is_err());
        assert!(resolve(&strings(&["cli"]), &strings(&["rust"])).is_err());
        assert!(resolve(&strings(&["nope"]), &[]).is_err());
    }
}
