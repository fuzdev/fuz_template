use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::anchors;
use crate::config::{MoltConfig, json_escape};
use crate::error::CliError;
use crate::features;
use crate::templates;

/// A single filesystem transformation in a molt plan. Paths are relative to
/// the repo root.
#[derive(Debug)]
pub enum Action {
    /// Replace `anchor` (which must appear exactly once) with `replacement`.
    ReplaceOnce {
        path: PathBuf,
        anchor: String,
        replacement: String,
        label: String,
    },
    /// Remove the one line starting with `prefix` (exactly one must), which
    /// must end with `suffix` (newline included), for lines whose middle
    /// churns, like a dependency's version.
    RemoveLine {
        path: PathBuf,
        prefix: String,
        suffix: String,
        label: String,
    },
    /// Replace every occurrence of `from` (which must appear at least once).
    ReplaceAll {
        path: PathBuf,
        from: String,
        to: String,
        label: String,
    },
    /// Replace the whole file; every `anchor` must appear in the current
    /// content (guarding against silent divergence from the template).
    ReplaceFile {
        path: PathBuf,
        anchors: Vec<String>,
        content: String,
        label: String,
    },
    /// Create a file; `path` must not exist yet.
    CreateFile {
        path: PathBuf,
        content: String,
        label: String,
    },
    /// Rename a directory; `to` must not exist yet.
    RenameDir { from: PathBuf, to: PathBuf },
    /// Delete a file.
    DeleteFile { path: PathBuf },
    /// Delete a directory recursively.
    DeleteDir { path: PathBuf },
}

impl Action {
    pub fn describe(&self) -> String {
        match self {
            Self::ReplaceOnce { path, label, .. }
            | Self::RemoveLine { path, label, .. }
            | Self::ReplaceAll { path, label, .. } => {
                format!("edit    {} — {label}", path.display())
            }
            Self::ReplaceFile { path, label, .. } => {
                format!("rewrite {} — {label}", path.display())
            }
            Self::CreateFile { path, label, .. } => {
                format!("create  {} — {label}", path.display())
            }
            Self::RenameDir { from, to } => {
                format!("rename  {}/ → {}/", from.display(), to.display())
            }
            Self::DeleteFile { path } => format!("delete  {}", path.display()),
            Self::DeleteDir { path } => format!("delete  {}/", path.display()),
        }
    }
}

fn replace_once(
    path: &str,
    anchor: &str,
    replacement: impl Into<String>,
    label: impl Into<String>,
) -> Action {
    Action::ReplaceOnce {
        path: PathBuf::from(path),
        anchor: anchor.to_owned(),
        replacement: replacement.into(),
        label: label.into(),
    }
}

/// The byte ranges of the lines in `content` that start with `prefix`, each
/// including its trailing newline when it has one.
pub fn lines_with_prefix(content: &str, prefix: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < content.len() {
        let end = content[start..]
            .find('\n')
            .map_or(content.len(), |i| start + i + 1);
        if content[start..end].starts_with(prefix) {
            ranges.push(start..end);
        }
        start = end;
    }
    ranges
}

/// Builds the full molt plan from resolved choices. Pure — reads nothing.
pub fn build_plan(config: &MoltConfig) -> Vec<Action> {
    let mut plan = Vec::new();
    let name = config.name.as_str();
    let npm_name = config.npm_name.as_str();

    // package.json identity
    plan.push(replace_once(
        "package.json",
        anchors::PACKAGE_JSON_NAME,
        format!("  \"name\": \"{}\",\n", json_escape(npm_name)),
        format!("name \u{2192} {npm_name}"),
    ));
    let description_replacement = if config.description.is_empty() {
        String::new()
    } else {
        format!(
            "  \"description\": \"{}\",\n",
            json_escape(&config.description)
        )
    };
    plan.push(replace_once(
        "package.json",
        anchors::PACKAGE_JSON_DESCRIPTION,
        description_replacement,
        "description",
    ));
    for (anchor, label) in [
        (anchors::PACKAGE_JSON_GLYPH, "remove template glyph"),
        (anchors::PACKAGE_JSON_LOGO, "remove template logo"),
        (anchors::PACKAGE_JSON_LOGO_ALT, "remove template logo_alt"),
        (
            anchors::PACKAGE_JSON_LICENSE,
            "remove license (choose your own)",
        ),
    ] {
        plan.push(replace_once("package.json", anchor, String::new(), label));
    }

    // the template's MIT license is fuz.dev's, not the new project's
    plan.push(Action::DeleteFile {
        path: PathBuf::from("LICENSE"),
    });

    // the TS twin ejector (`npm run molt`) is template machinery, deleted on
    // eject like molt's own crate — the script, its npm entry, and its check
    // test (which verifies anchors that no longer match once molted)
    plan.push(replace_once(
        "package.json",
        anchors::PACKAGE_JSON_MOLT_SCRIPT,
        String::new(),
        "remove the molt script",
    ));
    plan.push(Action::DeleteFile {
        path: PathBuf::from("src/lib/molt.ts"),
    });
    plan.push(Action::DeleteFile {
        path: PathBuf::from("src/test/molt.test.ts"),
    });
    plan.push(replace_once(
        ".gitattributes",
        anchors::GITATTRIBUTES_MOLT_NOTE,
        anchors::GITATTRIBUTES_MOLT_NOTE_REPLACEMENT,
        "drop molt's note from the LF rule",
    ));
    let homepage_replacement = config.domain.as_ref().map_or_else(String::new, |domain| {
        format!("  \"homepage\": \"https://{domain}/\",\n")
    });
    plan.push(replace_once(
        "package.json",
        anchors::PACKAGE_JSON_HOMEPAGE,
        homepage_replacement,
        "homepage",
    ));
    let repository_replacement = config.repo_url.as_ref().map_or_else(String::new, |url| {
        format!("  \"repository\": \"{}\",\n", json_escape(url))
    });
    plan.push(replace_once(
        "package.json",
        anchors::PACKAGE_JSON_REPOSITORY,
        repository_replacement,
        "repository",
    ));

    // custom domain
    if let Some(domain) = &config.domain {
        plan.push(Action::ReplaceFile {
            path: PathBuf::from("static/CNAME"),
            anchors: vec![anchors::CNAME_CONTENT.to_owned()],
            content: format!("{domain}\n"),
            label: format!("custom domain \u{2192} {domain}"),
        });
    } else {
        plan.push(Action::DeleteFile {
            path: PathBuf::from("static/CNAME"),
        });
    }

    // root layout: title + template logo
    plan.push(replace_once(
        "src/routes/+layout.svelte",
        anchors::LAYOUT_LOGO_IMPORT,
        String::new(),
        "remove template logo import",
    ));
    plan.push(replace_once(
        "src/routes/+layout.svelte",
        anchors::LAYOUT_SITE_STATE,
        anchors::LAYOUT_SITE_STATE_REPLACEMENT,
        "drop template icon",
    ));
    // the project name, not the npm name — a scoped `@you/app` reads badly
    // in a browser tab
    plan.push(replace_once(
        "src/routes/+layout.svelte",
        anchors::LAYOUT_TITLE,
        format!("<title>{name}</title>"),
        format!("title \u{2192} {name}"),
    ));

    // starter page + demo components
    let docs_link = if config.keeps(features::DOCS) {
        templates::PAGE_DOCS_LINK
    } else {
        ""
    };
    plan.push(Action::ReplaceFile {
        path: PathBuf::from("src/routes/+page.svelte"),
        anchors: vec![
            anchors::PAGE_MREOWS_IMPORT.to_owned(),
            anchors::H1_FUZ_TEMPLATE.to_owned(),
        ],
        content: templates::render(
            templates::PAGE_SVELTE,
            &[("__NAME__", name), ("__DOCS_LINK__", docs_link)],
        ),
        label: "minimal starter page".to_owned(),
    });
    plan.push(replace_once(
        "src/routes/about/+page.svelte",
        anchors::H1_FUZ_TEMPLATE,
        format!("<h1 class=\"mt_xl2\">{name}</h1>"),
        format!("heading \u{2192} {name}"),
    ));
    plan.push(Action::DeleteFile {
        path: PathBuf::from("src/lib/Mreows.svelte"),
    });
    plan.push(Action::DeleteFile {
        path: PathBuf::from("src/lib/Positioned.svelte"),
    });
    // the API docs prerender a page per `src/lib/` module, so deleting the
    // demo components would leave the docs build with no pages to render
    if config.keeps(features::DOCS) {
        plan.push(Action::CreateFile {
            path: PathBuf::from("src/lib/example.ts"),
            content: templates::render(templates::EXAMPLE_TS, &[("__NAME__", name)]),
            label: "starter module (the API docs need one)".to_owned(),
        });
    }

    // docs system, and the svelte-docinfo tooling that exists only for it
    if !config.keeps(features::DOCS) {
        plan.push(Action::DeleteDir {
            path: PathBuf::from("src/routes/docs"),
        });
        plan.push(Action::DeleteFile {
            path: PathBuf::from("src/routes/library.ts"),
        });
        plan.push(Action::RemoveLine {
            path: PathBuf::from("package.json"),
            prefix: anchors::PACKAGE_JSON_SVELTE_DOCINFO.to_owned(),
            suffix: anchors::PACKAGE_JSON_SVELTE_DOCINFO_SUFFIX.to_owned(),
            label: "remove the svelte-docinfo devDependency".to_owned(),
        });
        plan.push(replace_once(
            "vite.config.ts",
            anchors::VITE_DOCINFO_IMPORT,
            String::new(),
            "remove the svelte-docinfo import",
        ));
        plan.push(replace_once(
            "vite.config.ts",
            anchors::VITE_DOCINFO_PLUGIN,
            String::new(),
            "remove the svelte-docinfo plugin",
        ));
        plan.push(replace_once(
            "src/app.d.ts",
            anchors::APP_D_TS_DOCINFO,
            String::new(),
            "remove the svelte-docinfo ambient types",
        ));
    }

    // regenerated docs
    let description_block = if config.description.is_empty() {
        String::new()
    } else {
        format!("> {}\n\n", config.description)
    };
    let (readme_rust, claude_rust) = if config.keeps(features::RUST) {
        (
            templates::README_RUST_SECTION,
            templates::CLAUDE_RUST_SECTION,
        )
    } else {
        ("", "")
    };
    let claude_docs_bullet = if config.keeps(features::DOCS) {
        templates::CLAUDE_DOCS_BULLET
    } else {
        ""
    };
    plan.push(Action::ReplaceFile {
        path: PathBuf::from("README.md"),
        anchors: vec![anchors::README_H1.to_owned()],
        content: templates::render(
            templates::README_MD,
            &[
                ("__NPM_NAME__", npm_name),
                ("__DESCRIPTION_BLOCK__", &description_block),
                ("__RUST_SECTION__", readme_rust),
            ],
        ),
        label: "regenerate for the new project".to_owned(),
    });
    plan.push(Action::ReplaceFile {
        path: PathBuf::from("CLAUDE.md"),
        anchors: vec![anchors::CLAUDE_H1.to_owned()],
        content: templates::render(
            templates::CLAUDE_MD,
            &[
                ("__NAME__", name),
                ("__DESCRIPTION_BLOCK__", &description_block),
                ("__DOCS_BULLET__", claude_docs_bullet),
                ("__RUST_SECTION__", claude_rust),
            ],
        ),
        label: "regenerate for the new project (AGENTS.md symlinks here)".to_owned(),
    });

    // .github extras: personalized when kept (the template's funding handles
    // and discussion links must never ship in someone else's project)
    if config.keeps(features::GITHUB_EXTRAS) {
        plan.push(Action::ReplaceFile {
            path: PathBuf::from(".github/FUNDING.yml"),
            anchors: vec![anchors::FUNDING_GITHUB.to_owned()],
            content: templates::FUNDING_YML.to_owned(),
            label: "funding placeholders (fill in or delete)".to_owned(),
        });
        if let Some(repo_url) = &config.repo_url {
            for path in [
                ".github/ISSUE_TEMPLATE/config.yml",
                ".github/ISSUE_TEMPLATE/preapproved.md",
            ] {
                plan.push(Action::ReplaceAll {
                    path: PathBuf::from(path),
                    from: anchors::TEMPLATE_REPO_URL.to_owned(),
                    to: repo_url.clone(),
                    label: format!("discussions url \u{2192} {repo_url}"),
                });
            }
        }
    } else {
        plan.push(Action::DeleteFile {
            path: PathBuf::from(".github/FUNDING.yml"),
        });
        plan.push(Action::DeleteDir {
            path: PathBuf::from(".github/ISSUE_TEMPLATE"),
        });
    }

    // the Rust workspace, and molt's own crate; `cli` is always kept here —
    // `resolve_config` rejects a kept `rust` with an empty member group,
    // since cargo refuses to load an empty workspace
    if config.keeps(features::RUST) {
        let members = format!("\"crates/{name}\"");
        plan.push(Action::ReplaceFile {
            path: PathBuf::from("Cargo.toml"),
            anchors: vec![anchors::WORKSPACE_MEMBERS.to_owned()],
            content: templates::render(
                templates::WORKSPACE_CARGO_TOML,
                &[("__MEMBERS__", &members), ("__LICENSE__", "")],
            ),
            label: "workspace without molt's crate or the template's license".to_owned(),
        });
        plan.push(replace_once(
            "crates/app_cli/Cargo.toml",
            anchors::APP_CLI_LICENSE,
            String::new(),
            "remove the license inheritance (the workspace line is gone)",
        ));
        // rename the token before inserting the user's description, which may
        // itself contain "app_cli" and must survive verbatim
        for path in [
            "crates/app_cli/Cargo.toml",
            "crates/app_cli/src/main.rs",
            "crates/app_cli/src/error.rs",
        ] {
            plan.push(Action::ReplaceAll {
                path: PathBuf::from(path),
                from: anchors::APP_CLI_TOKEN.to_owned(),
                to: name.to_owned(),
                label: format!("{} \u{2192} {name}", anchors::APP_CLI_TOKEN),
            });
        }
        let description_replacement = if config.description.is_empty() {
            String::new()
        } else {
            format!("description = \"{}\"\n", json_escape(&config.description))
        };
        plan.push(replace_once(
            "crates/app_cli/Cargo.toml",
            anchors::APP_CLI_DESCRIPTION,
            description_replacement,
            "description",
        ));
        plan.push(Action::RenameDir {
            from: PathBuf::from("crates/app_cli"),
            to: PathBuf::from(format!("crates/{name}")),
        });
        plan.push(Action::DeleteDir {
            path: PathBuf::from("crates/molt"),
        });
        plan.push(replace_once(
            ".github/workflows/check.yml",
            anchors::CI_RUST_JOB_MOLT_COMMENT,
            String::new(),
            "remove molt's note from the rust job",
        ));
    } else {
        plan.push(replace_once(
            ".github/workflows/check.yml",
            anchors::CI_RUST_JOB,
            String::new(),
            "remove the rust job",
        ));
        for path in [
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "clippy.toml",
        ] {
            plan.push(Action::DeleteFile {
                path: PathBuf::from(path),
            });
        }
        plan.push(Action::DeleteDir {
            path: PathBuf::from("crates"),
        });
    }
    plan.push(Action::DeleteDir {
        path: PathBuf::from(".cargo"),
    });

    // deletes run after every edit and rename, so a mid-apply failure on the
    // `--force` dirty path (the one with no clean undo point) strands as
    // little as possible
    let (deletes, mut ordered): (Vec<_>, Vec<_>) = plan
        .into_iter()
        .partition(|action| matches!(action, Action::DeleteFile { .. } | Action::DeleteDir { .. }));
    ordered.extend(deletes);
    ordered
}

/// Verifies every action's preconditions against the tree at `root`,
/// returning human-readable issues (empty = the plan is applicable).
pub fn verify(root: &Path, plan: &[Action]) -> Result<Vec<String>, CliError> {
    let mut issues = Vec::new();
    for action in plan {
        match action {
            Action::ReplaceOnce { path, anchor, .. } => match read(root, path)? {
                Some(content) => {
                    let count = content.matches(anchor.as_str()).count();
                    if count != 1 {
                        issues.push(format!(
                            "{}: anchor matched {count} times (expected exactly 1): {anchor:?}",
                            path.display()
                        ));
                    }
                }
                None => issues.push(format!("{}: file missing", path.display())),
            },
            Action::RemoveLine {
                path,
                prefix,
                suffix,
                ..
            } => match read(root, path)? {
                Some(content) => {
                    let lines = lines_with_prefix(&content, prefix);
                    if lines.len() != 1 {
                        issues.push(format!(
                            "{}: line prefix matched {} lines (expected exactly 1): {prefix:?}",
                            path.display(),
                            lines.len()
                        ));
                    } else if !content[lines[0].clone()].ends_with(suffix.as_str()) {
                        issues.push(format!(
                            "{}: the line starting with {prefix:?} doesn't end with {suffix:?}",
                            path.display()
                        ));
                    }
                }
                None => issues.push(format!("{}: file missing", path.display())),
            },
            Action::ReplaceAll { path, from, .. } => match read(root, path)? {
                Some(content) => {
                    if !content.contains(from.as_str()) {
                        issues.push(format!(
                            "{}: expected occurrences of {from:?}, found none",
                            path.display()
                        ));
                    }
                }
                None => issues.push(format!("{}: file missing", path.display())),
            },
            Action::ReplaceFile { path, anchors, .. } => match read(root, path)? {
                Some(content) => {
                    for anchor in anchors {
                        if !content.contains(anchor.as_str()) {
                            issues.push(format!(
                                "{}: expected content not found: {anchor:?}",
                                path.display()
                            ));
                        }
                    }
                }
                None => issues.push(format!("{}: file missing", path.display())),
            },
            // a create target in the way is `conflicts`' job — its fix is
            // moving the file aside, not restoring it
            Action::CreateFile { .. } => {}
            Action::RenameDir { from, to } => {
                if !root.join(from).is_dir() {
                    issues.push(format!(
                        "{}: expected a directory to rename",
                        from.display()
                    ));
                }
                if root.join(to).exists() {
                    issues.push(format!("{}: rename target already exists", to.display()));
                }
            }
            Action::DeleteFile { path } => {
                let full = root.join(path);
                if !full.is_file() {
                    issues.push(format!("{}: expected a file to delete", path.display()));
                }
            }
            Action::DeleteDir { path } => {
                let full = root.join(path);
                if !full.is_dir() {
                    issues.push(format!(
                        "{}: expected a directory to delete",
                        path.display()
                    ));
                }
            }
        }
    }
    Ok(issues)
}

/// Lists the files the plan creates that already exist at `root`, kept apart
/// from `verify`'s drift because the remedy differs.
pub fn conflicts(root: &Path, plan: &[Action]) -> Vec<String> {
    plan.iter()
        .filter_map(|action| match action {
            Action::CreateFile { path, .. } if root.join(path).exists() => Some(format!(
                "{}: already exists, and molt creates it",
                path.display()
            )),
            _ => None,
        })
        .collect()
}

fn read(root: &Path, path: &Path) -> Result<Option<String>, CliError> {
    let full = root.join(path);
    match fs::read_to_string(&full) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliError::Io { path: full, source }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_with_prefix_matches_whole_lines() {
        let content = "{\n  \"a\": \"^1.0.0\",\nx  \"a\": \"mid\",\n  \"a\": \"^2.0.0\"";
        // only line starts count, not the prefix mid-line
        let ranges = lines_with_prefix(content, "  \"a\": \"");
        assert_eq!(ranges.len(), 2);
        assert_eq!(&content[ranges[0].clone()], "  \"a\": \"^1.0.0\",\n");
        // the last line has no trailing newline
        assert_eq!(&content[ranges[1].clone()], "  \"a\": \"^2.0.0\"");
        assert!(lines_with_prefix(content, "  \"c\": ").is_empty());
        assert!(lines_with_prefix("", "x").is_empty());
    }

    #[test]
    fn remove_line_requires_exactly_one_match() {
        let dir = std::env::temp_dir().join(format!(
            "fuz_template_molt_test_remove_line_{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let plan = [Action::RemoveLine {
            path: PathBuf::from("package.json"),
            prefix: "  \"a\": \"".to_owned(),
            suffix: "\",\n".to_owned(),
            label: "remove a".to_owned(),
        }];
        for (content, expected_issues) in [
            ("{\n  \"a\": \"^1.0.0\",\n  \"b\": \"^1.0.0\"\n}\n", 0),
            ("{\n  \"b\": \"^1.0.0\"\n}\n", 1),
            ("{\n  \"a\": \"^1.0.0\",\n  \"a\": \"^2.0.0\"\n}\n", 1),
            // the last entry has no trailing comma — removing it would leave
            // the previous line's comma dangling, so it's refused
            ("{\n  \"b\": \"^1.0.0\",\n  \"a\": \"^1.0.0\"\n}\n", 1),
            // a CRLF line ends in `,\r\n` — refused like every other anchor
            // under CRLF (`.gitattributes` forces LF)
            (
                "{\r\n  \"a\": \"^1.0.0\",\r\n  \"b\": \"^1.0.0\"\r\n}\r\n",
                1,
            ),
        ] {
            fs::write(dir.join("package.json"), content).unwrap();
            assert_eq!(
                verify(&dir, &plan).unwrap().len(),
                expected_issues,
                "{content:?}"
            );
        }
        fs::write(
            dir.join("package.json"),
            "{\n  \"a\": \"^1.0.0\",\n  \"b\": \"^1.0.0\"\n}\n",
        )
        .unwrap();
        crate::apply::apply(&dir, &plan).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("package.json")).unwrap(),
            "{\n  \"b\": \"^1.0.0\"\n}\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn create_file_conflicts_are_not_drift() {
        let dir = std::env::temp_dir().join(format!(
            "fuz_template_molt_test_create_file_{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let plan = [Action::CreateFile {
            path: PathBuf::from("example.ts"),
            content: String::new(),
            label: "create".to_owned(),
        }];
        assert!(conflicts(&dir, &plan).is_empty());
        fs::write(dir.join("example.ts"), "").unwrap();
        assert_eq!(conflicts(&dir, &plan).len(), 1);
        assert!(verify(&dir, &plan).unwrap().is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
