use std::fs;
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
            Self::ReplaceOnce { path, label, .. } | Self::ReplaceAll { path, label, .. } => {
                format!("edit    {} — {label}", path.display())
            }
            Self::ReplaceFile { path, label, .. } => {
                format!("rewrite {} — {label}", path.display())
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
    ] {
        plan.push(replace_once("package.json", anchor, String::new(), label));
    }
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
    plan.push(replace_once(
        "src/routes/+layout.svelte",
        anchors::LAYOUT_TITLE,
        format!("<title>{npm_name}</title>"),
        format!("title \u{2192} {npm_name}"),
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

    // docs system, and the svelte-docinfo tooling that exists only for it
    if !config.keeps(features::DOCS) {
        plan.push(Action::DeleteDir {
            path: PathBuf::from("src/routes/docs"),
        });
        plan.push(Action::DeleteFile {
            path: PathBuf::from("src/routes/library.ts"),
        });
        plan.push(replace_once(
            "package.json",
            anchors::PACKAGE_JSON_SVELTE_DOCINFO,
            String::new(),
            "remove the svelte-docinfo devDependency",
        ));
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
    // `resolve_config` rejects a kept `rust` with no member crates, since
    // cargo refuses to load an empty workspace
    if config.keeps(features::RUST) {
        let members = format!("\"crates/{name}\"");
        plan.push(Action::ReplaceFile {
            path: PathBuf::from("Cargo.toml"),
            anchors: vec![anchors::WORKSPACE_MEMBERS.to_owned()],
            content: templates::render(
                templates::WORKSPACE_CARGO_TOML,
                &[("__MEMBERS__", &members)],
            ),
            label: "workspace without molt's crate".to_owned(),
        });
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
        plan.push(Action::RenameDir {
            from: PathBuf::from("crates/app_cli"),
            to: PathBuf::from(format!("crates/{name}")),
        });
        plan.push(Action::DeleteDir {
            path: PathBuf::from("crates/fuz_template"),
        });
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

    plan
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

fn read(root: &Path, path: &Path) -> Result<Option<String>, CliError> {
    let full = root.join(path);
    match fs::read_to_string(&full) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliError::Io { path: full, source }),
    }
}
