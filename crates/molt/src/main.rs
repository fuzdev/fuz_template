//! molt — transforms this `fuz_template` clone into your own project, then
//! deletes itself. See the repo `README.md` for details.

mod anchors;
mod apply;
mod check;
mod cli;
mod config;
mod error;
mod features;
mod git;
mod plan;
mod templates;
mod wizard;

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use argh::FromArgs as _;

use crate::cli::TopLevel;
use crate::config::MoltConfig;
use crate::error::CliError;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let bin = args.first().map_or("molt", String::as_str);
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    // Parse explicitly rather than `argh::from_env`, which hard-exits `1` on any
    // parse failure — a usage error is `2` by shell convention.
    let top = match TopLevel::from_args(&[bin], &rest) {
        Ok(top) => top,
        Err(early_exit) => {
            return if early_exit.status.is_ok() {
                println!("{}", early_exit.output);
                ExitCode::SUCCESS
            } else {
                eprintln!("{}", early_exit.output);
                ExitCode::from(2)
            };
        }
    };
    match run(&top) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            if let Some(hint) = e.hint() {
                eprintln!("hint: {hint}");
            }
            ExitCode::from(e.exit_code())
        }
    }
}

fn run(top: &TopLevel) -> Result<ExitCode, CliError> {
    let root = locate_root()?;
    if top.subcommand.is_some() {
        if top.has_molt_flags() {
            return Err(CliError::Usage(
                "`molt check` takes no other flags".to_owned(),
            ));
        }
        check::run(&root)
    } else {
        molt(top, &root)
    }
}

/// Walks up from the current directory to the template's repo root.
fn locate_root() -> Result<PathBuf, CliError> {
    let mut dir = env::current_dir().map_err(|source| CliError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    loop {
        if dir.join("package.json").is_file() && dir.join("crates/molt").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(CliError::precondition(
                "not inside the fuz_template repo (no package.json + crates/molt found)",
                Some("run `cargo molt` from your clone of fuz_template"),
            ));
        }
    }
}

/// What stands between a printed plan and applying it, given the run mode.
///
/// A terminal always gets a confirm prompt — the wizard's answers were just
/// typed, and one keystroke catches a typo'd name before it hits disk. The
/// one combination with no gate at all is `--wetrun` on a clean tree without
/// a terminal — there `git reset --hard && git clean -fd` restores the
/// pre-molt state (the tree was clean, so `git clean` removes only files
/// molt created). A dirty tree (reachable only via `--force`) never applies
/// without the dirty-specific in-the-moment confirmation, and without a
/// terminal it never applies at all: "commit first" is always available, so
/// an override flag would just recreate the hole.
#[derive(Debug, PartialEq, Eq)]
enum ApplyGate {
    /// Apply without further confirmation (clean tree + `--wetrun`, no terminal).
    Apply,
    /// Ask the standard confirm prompt.
    Confirm,
    /// Ask a scarier prompt: applying with no clean undo point.
    ConfirmDirty,
    /// Print the dry-run note and stop.
    DryRun,
    /// Refuse: destructive apply on a dirty tree needs a terminal (exit 2).
    RefuseDirty,
}

const fn apply_gate(wetrun: bool, clean: bool, interactive: bool) -> ApplyGate {
    match (wetrun, clean, interactive) {
        (_, true, true) => ApplyGate::Confirm,
        (_, false, true) => ApplyGate::ConfirmDirty,
        (true, true, false) => ApplyGate::Apply,
        (true, false, false) => ApplyGate::RefuseDirty,
        (false, _, false) => ApplyGate::DryRun,
    }
}

fn molt(top: &TopLevel, root: &Path) -> Result<ExitCode, CliError> {
    let interactive = wizard::interactive();

    if !root.join(".git").exists() {
        return Err(CliError::precondition(
            "not a git repository — molt refuses to run without an undo path",
            Some("git init && git add -A && git commit -m 'init from fuz_template'"),
        ));
    }
    // a failed `git status` is its own problem, not a dirty tree
    let clean = match git::output(root, &["status", "--porcelain"])? {
        Some(out) => out.trim().is_empty(),
        None => {
            return Err(CliError::precondition(
                "`git status` failed in this repo",
                Some("make sure `git status --porcelain` succeeds here, then rerun molt"),
            ));
        }
    };
    if !clean && !top.force {
        return Err(CliError::precondition(
            "the git tree is dirty — molt wants a clean tree so it stays undoable",
            Some("commit or stash your changes, or pass --force to proceed anyway"),
        ));
    }
    let gate = apply_gate(top.wetrun, clean, interactive);
    if gate == ApplyGate::RefuseDirty {
        // refuse before prompting/planning — this is an invocation problem
        return Err(CliError::precondition(
            "refusing to apply to a dirty git tree without a terminal — there would be no clean undo point",
            Some("commit or stash first, or run interactively to confirm the dirty apply"),
        ));
    }

    let config = resolve_config(top, root, interactive)?;
    let plan = plan::build_plan(&config);
    let issues = plan::verify(root, &plan)?;
    if !issues.is_empty() {
        return Err(CliError::Drift(issues.join("\n")));
    }

    println!("\nmolt plan ({} actions):", plan.len());
    for action in &plan {
        println!("  {}", action.describe());
    }

    let apply_now = match gate {
        ApplyGate::Apply => true,
        ApplyGate::ConfirmDirty => {
            println!();
            wizard::prompt_bool(
                "the git tree is DIRTY — apply anyway, with no clean undo point?",
                false,
            )?
        }
        ApplyGate::Confirm => {
            println!();
            wizard::prompt_bool(
                "apply this plan? the template becomes your project and molt deletes itself",
                false,
            )?
        }
        ApplyGate::DryRun => {
            println!("\ndry run — nothing written. pass --wetrun to apply.");
            false
        }
        // handled above, before planning
        ApplyGate::RefuseDirty => unreachable!(),
    };
    if !apply_now {
        if gate != ApplyGate::DryRun {
            println!("declined — nothing written");
        }
        return Ok(ExitCode::SUCCESS);
    }

    if matches!(gate, ApplyGate::Confirm | ApplyGate::ConfirmDirty) {
        // the tree may have changed while the prompt waited — apply would
        // silently skip an edit whose anchor disappeared, so verify again
        let issues = plan::verify(root, &plan)?;
        if !issues.is_empty() {
            return Err(CliError::Drift(issues.join("\n")));
        }
    }
    apply::apply(root, &plan)?;
    print_next_steps(&config, clean);
    Ok(ExitCode::SUCCESS)
}

fn resolve_config(top: &TopLevel, root: &Path, interactive: bool) -> Result<MoltConfig, CliError> {
    let name = match &top.name {
        Some(name) => {
            config::validate_name(name)?;
            name.clone()
        }
        None if interactive => {
            wizard::prompt_validated("project name", None, config::validate_name)?
        }
        None => {
            return Err(CliError::Usage(
                "--name is required when not running interactively".to_owned(),
            ));
        }
    };

    let npm_name = match &top.npm_name {
        Some(npm_name) => {
            config::validate_npm_name(npm_name)?;
            npm_name.clone()
        }
        None if interactive => {
            wizard::prompt_validated("npm package name", Some(&name), config::validate_npm_name)?
        }
        None => name.clone(),
    };

    let description = match &top.description {
        Some(description) => {
            let description = description.trim().to_owned();
            config::validate_description(&description)?;
            description
        }
        None if interactive => wizard::prompt("one-line description (optional)", Some(""))?,
        None => String::new(),
    };

    let domain = match &top.domain {
        Some(domain) => {
            let domain = non_empty(domain);
            if let Some(domain) = &domain {
                config::validate_domain(domain)?;
            }
            domain
        }
        None if interactive => non_empty(&wizard::prompt_validated(
            "custom domain like example.com (optional; sets CNAME + homepage)",
            Some(""),
            |value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Ok(())
                } else {
                    config::validate_domain(trimmed)
                }
            },
        )?),
        None => None,
    };

    let derived_repo = git::output(root, &["remote", "get-url", "origin"])?
        .and_then(|url| git::normalize_remote_url(&url));
    let repo_url = match &top.repo {
        Some(repo) => non_empty(repo),
        None if interactive => non_empty(&wizard::prompt(
            "repository url (optional)",
            derived_repo.as_deref(),
        )?),
        None => derived_repo,
    };

    let (mut kept, explicit) = features::resolve(&top.keep, &top.strip)?;
    if interactive {
        // registry order puts parents before dependents, so `requires` and
        // `member_of` parents are already decided when a dependent comes up
        for feature in &features::FEATURES {
            if explicit.contains(feature.id) {
                continue;
            }
            if let Some(parent) = feature.requires
                && !kept.contains(parent)
            {
                kept.remove(feature.id);
                continue;
            }
            // a prompt whose answer explicit flags already force is skipped
            // with a note instead of contradicting the flag after the fact
            if let Some(child) = features::FEATURES.iter().find(|f| {
                f.requires == Some(feature.id) && explicit.contains(f.id) && kept.contains(f.id)
            }) {
                println!(
                    "note: keeping {} — --keep {} needs it",
                    feature.id, child.id
                );
                kept.insert(feature.id);
                continue;
            }
            // a parent whose members were all explicitly stripped can't be
            // kept (e.g. cargo rejects a workspace with no member crates)
            if features::members_of(feature.id).next().is_some()
                && features::members_of(feature.id)
                    .all(|m| explicit.contains(m.id) && !kept.contains(m.id))
            {
                let members = features::members_of(feature.id)
                    .map(|m| m.id)
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "note: --strip {members} leaves {} without a required member — stripping it too",
                    feature.id
                );
                kept.remove(feature.id);
                continue;
            }
            // the sole member of a kept parent rides with the parent's
            // prompt — the parent can't be kept without it, so there is no
            // separate decision to prompt for
            if let Some(parent) = feature.member_of
                && kept.contains(parent)
                && features::members_of(parent).count() == 1
            {
                kept.insert(feature.id);
                continue;
            }
            if wizard::prompt_bool(feature.prompt, feature.default_keep)? {
                kept.insert(feature.id);
            } else {
                kept.remove(feature.id);
            }
        }
        features::cascade(&mut kept);
        // a kept parent with every member declined can't build — repair the
        // wizard case (all choices came from prompts); explicit flags are
        // rejected below instead. unreachable while every group has a sole
        // member (the wizard skips that prompt); kept for when a second
        // member returns the prompts
        for parent in features::empty_groups(&kept) {
            if explicit.contains(parent)
                || features::members_of(parent).any(|m| explicit.contains(m.id))
            {
                continue;
            }
            println!("note: declining every member of {parent} leaves it empty — stripping it too");
            kept.remove(parent);
            features::cascade(&mut kept);
        }
    }
    if let Some(&parent) = features::empty_groups(&kept).first() {
        let members = features::members_of(parent)
            .map(|m| m.id)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(CliError::Usage(format!(
            "keeping {parent} requires at least one of its member features ({members}) — keep one, or strip {parent} too"
        )));
    }

    Ok(MoltConfig {
        name,
        npm_name,
        description,
        domain,
        repo_url,
        kept,
    })
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn print_next_steps(config: &MoltConfig, clean: bool) {
    println!(
        "\nmolt complete — the project is now {}. next steps:",
        config.name
    );
    println!("  git status   # review what changed");
    println!("  npm i        # refresh package-lock.json for the new name");
    println!("  gro check    # typecheck, test, lint, format");
    if config.keeps(features::RUST) {
        println!("  cargo check  # refresh Cargo.lock for your crate");
    }
    println!(
        "  git add -A && git commit -m \"chore: molt fuz_template into {}\"",
        config.name
    );
    if clean {
        println!("\nto undo the molt: git reset --hard && git clean -fd");
    }
    println!(
        "\nstatic/logo.svg and static/favicon.png still carry the template's spider — replace them when ready."
    );
    println!(
        "molt deleted the template's MIT LICENSE and license fields — choose your own: https://choosealicense.com/"
    );
    if config.keeps(features::GITHUB_EXTRAS) {
        if config.repo_url.is_some() {
            println!(
                ".github/FUNDING.yml now holds placeholder funding links — fill in or delete."
            );
        } else {
            println!(
                ".github/FUNDING.yml now holds placeholder funding links, and the issue-template discussion links still point at the template (no repo url to derive) — update or delete them."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_gate_only_headless_clean_wetrun_applies_ungated() {
        assert_eq!(apply_gate(true, true, false), ApplyGate::Apply);
        // a terminal always confirms, even with --wetrun — the wizard's
        // answers were just typed, so one keystroke catches a typo
        assert_eq!(apply_gate(true, true, true), ApplyGate::Confirm);
        assert_eq!(apply_gate(false, true, true), ApplyGate::Confirm);
        // a dirty tree never applies without the dirty-specific confirmation
        assert_eq!(apply_gate(true, false, true), ApplyGate::ConfirmDirty);
        assert_eq!(apply_gate(false, false, true), ApplyGate::ConfirmDirty);
        // ...and never at all without a terminal
        assert_eq!(apply_gate(true, false, false), ApplyGate::RefuseDirty);
        // non-interactive without --wetrun never writes
        assert_eq!(apply_gate(false, true, false), ApplyGate::DryRun);
        assert_eq!(apply_gate(false, false, false), ApplyGate::DryRun);
    }
}
