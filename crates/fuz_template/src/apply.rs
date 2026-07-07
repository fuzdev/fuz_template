use std::fs;
use std::path::Path;

use crate::error::CliError;
use crate::plan::Action;

/// Applies a verified plan at `root`. Callers must run `plan::verify` first —
/// apply assumes anchors match and targets exist.
pub fn apply(root: &Path, plan: &[Action]) -> Result<(), CliError> {
    for action in plan {
        match action {
            Action::ReplaceOnce {
                path,
                anchor,
                replacement,
                ..
            } => {
                let full = root.join(path);
                let content = fs::read_to_string(&full).map_err(|source| CliError::Io {
                    path: full.clone(),
                    source,
                })?;
                let updated = content.replacen(anchor.as_str(), replacement, 1);
                fs::write(&full, updated).map_err(|source| CliError::Io { path: full, source })?;
            }
            Action::ReplaceAll { path, from, to, .. } => {
                let full = root.join(path);
                let content = fs::read_to_string(&full).map_err(|source| CliError::Io {
                    path: full.clone(),
                    source,
                })?;
                let updated = content.replace(from.as_str(), to);
                fs::write(&full, updated).map_err(|source| CliError::Io { path: full, source })?;
            }
            Action::ReplaceFile { path, content, .. } => {
                let full = root.join(path);
                fs::write(&full, content).map_err(|source| CliError::Io { path: full, source })?;
            }
            Action::RenameDir { from, to } => {
                let from_full = root.join(from);
                let to_full = root.join(to);
                fs::rename(&from_full, &to_full).map_err(|source| CliError::Io {
                    path: from_full,
                    source,
                })?;
            }
            Action::DeleteFile { path } => {
                let full = root.join(path);
                fs::remove_file(&full).map_err(|source| CliError::Io { path: full, source })?;
            }
            Action::DeleteDir { path } => {
                let full = root.join(path);
                fs::remove_dir_all(&full).map_err(|source| CliError::Io { path: full, source })?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::check::sample_configs;
    use crate::plan::{build_plan, verify};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    /// Copies the parts of the repo that molt touches into a scratch dir.
    fn copy_template(destination: &Path) {
        let root = repo_root();
        for dir in ["src", "static", ".github", ".cargo", "crates"] {
            copy_dir(&root.join(dir), &destination.join(dir));
        }
        for file in [
            "package.json",
            "README.md",
            "CLAUDE.md",
            "LICENSE",
            "vite.config.ts",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "clippy.toml",
        ] {
            fs::copy(root.join(file), destination.join(file)).unwrap();
        }
    }

    fn copy_dir(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                copy_dir(&entry.path(), &target);
            } else if file_type.is_file() {
                fs::copy(entry.path(), &target).unwrap();
            }
            // symlinks (e.g. AGENTS.md at the root) aren't under the copied dirs
        }
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fuz_template_molt_test_{label}_{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read(root: &Path, path: &str) -> String {
        fs::read_to_string(root.join(path)).unwrap()
    }

    #[test]
    fn apply_keep_rust_sample() {
        let [config, _] = sample_configs();
        let dir = scratch_dir("keep_rust");
        copy_template(&dir);

        let plan = build_plan(&config);
        let issues = verify(&dir, &plan).unwrap();
        assert!(issues.is_empty(), "verify issues: {issues:#?}");
        apply(&dir, &plan).unwrap();

        let package_json = read(&dir, "package.json");
        assert!(package_json.contains("\"name\": \"@sample/sample_app\""));
        assert!(!package_json.contains("glyph"));
        assert!(package_json.contains("\"homepage\": \"https://sample.example.com/\""));
        assert_eq!(read(&dir, "static/CNAME"), "sample.example.com\n");

        // the template's MIT license never ships in a molted project
        assert!(!dir.join("LICENSE").exists());
        assert!(!package_json.contains("\"license\""));

        let layout = read(&dir, "src/routes/+layout.svelte");
        assert!(!layout.contains("logo_fuz_template"));
        assert!(layout.contains("<title>@sample/sample_app</title>"));

        let page = read(&dir, "src/routes/+page.svelte");
        assert!(!page.contains("Mreows"));
        assert!(page.contains("<h1>sample_app</h1>"));
        assert!(page.contains("resolve('/docs')"));
        assert!(!dir.join("src/lib/Mreows.svelte").exists());
        assert!(!dir.join("src/lib/Positioned.svelte").exists());

        // docs kept
        assert!(dir.join("src/routes/docs").is_dir());
        assert!(dir.join("src/routes/library.ts").is_file());

        assert!(read(&dir, "README.md").starts_with("# @sample/sample_app"));
        let claude = read(&dir, "CLAUDE.md");
        assert!(claude.starts_with("# sample_app"));
        assert!(claude.contains("## Rust workspace"));
        assert!(claude.contains("src/routes/docs"));

        // github-extras kept: personalized, never the template author's links
        let funding = read(&dir, ".github/FUNDING.yml");
        assert!(!funding.contains("ryanatkn"));
        assert!(funding.contains("your-github-username"));
        let issue_config = read(&dir, ".github/ISSUE_TEMPLATE/config.yml");
        assert!(
            issue_config.contains("https://github.com/sample/sample_app/discussions/new/choose")
        );
        assert!(!issue_config.contains("fuzdev/fuz_template"));
        assert!(
            !read(&dir, ".github/ISSUE_TEMPLATE/preapproved.md").contains("fuzdev/fuz_template")
        );
        assert!(read(&dir, ".github/workflows/check.yml").contains("cargo clippy"));

        // docs kept: the svelte-docinfo tooling stays
        assert!(package_json.contains("svelte-docinfo"));
        assert!(read(&dir, "vite.config.ts").contains("svelte_docinfo()"));

        let workspace = read(&dir, "Cargo.toml");
        assert!(workspace.contains("members = [\"crates/sample_app\"]"));
        assert!(!workspace.contains("license"));
        assert!(!dir.join("crates/fuz_template").exists());
        assert!(!dir.join("crates/app_cli").exists());
        assert!(!dir.join(".cargo").exists());
        let crate_manifest = read(&dir, "crates/sample_app/Cargo.toml");
        assert!(crate_manifest.contains("name = \"sample_app\""));
        // the user's description survives verbatim — the app_cli token rename
        // runs before the description insert
        assert!(crate_manifest.contains("description = \"a sample app that replaces app_cli\""));
        assert!(!crate_manifest.contains("license"));
        let main_rs = read(&dir, "crates/sample_app/src/main.rs");
        assert!(main_rs.contains("hello {who}, from sample_app"));
        assert!(!main_rs.contains("app_cli"));
        assert!(!read(&dir, "crates/sample_app/src/error.rs").contains("app_cli"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn apply_strip_rust_sample() {
        let [_, config] = sample_configs();
        let dir = scratch_dir("strip_rust");
        copy_template(&dir);

        let plan = build_plan(&config);
        let issues = verify(&dir, &plan).unwrap();
        assert!(issues.is_empty(), "verify issues: {issues:#?}");
        apply(&dir, &plan).unwrap();

        let package_json = read(&dir, "package.json");
        assert!(package_json.contains("\"name\": \"plain_app\""));
        assert!(!package_json.contains("homepage"));
        assert!(!package_json.contains("repository"));
        assert!(!package_json.contains("\"license\""));
        assert!(!dir.join("static/CNAME").exists());
        assert!(!dir.join("LICENSE").exists());

        assert!(!dir.join("Cargo.toml").exists());
        assert!(!dir.join("Cargo.lock").exists());
        assert!(!dir.join("crates").exists());
        assert!(!dir.join(".cargo").exists());
        assert!(!dir.join("rust-toolchain.toml").exists());
        assert!(!dir.join("clippy.toml").exists());

        let workflow = read(&dir, ".github/workflows/check.yml");
        assert!(!workflow.contains("cargo"));
        assert!(workflow.contains("npx @fuzdev/gro check"));

        // docs stripped, along with the svelte-docinfo tooling
        assert!(!dir.join("src/routes/docs").exists());
        assert!(!dir.join("src/routes/library.ts").exists());
        let page = read(&dir, "src/routes/+page.svelte");
        assert!(!page.contains("resolve('/docs')"));
        assert!(page.contains("resolve('/about')"));
        assert!(!package_json.contains("svelte-docinfo"));
        assert!(!read(&dir, "vite.config.ts").contains("svelte_docinfo"));
        assert!(!read(&dir, "src/app.d.ts").contains("svelte-docinfo"));

        // extras stripped in this sample
        assert!(!dir.join(".github/FUNDING.yml").exists());
        assert!(!dir.join(".github/ISSUE_TEMPLATE").exists());

        let claude = read(&dir, "CLAUDE.md");
        assert!(!read(&dir, "README.md").contains("## rust"));
        assert!(!claude.contains("## Rust workspace"));
        assert!(!claude.contains("src/routes/docs"));

        fs::remove_dir_all(&dir).unwrap();
    }
}
