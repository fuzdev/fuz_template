use std::path::Path;
use std::process::Command;

use crate::error::CliError;

/// Runs a git command at `root`, returning its stdout on success and `None`
/// on a nonzero exit (e.g. no `origin` remote configured).
pub fn output(root: &Path, args: &[&str]) -> Result<Option<String>, CliError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|source| {
            CliError::precondition(
                format!("failed to run git: {source}"),
                Some("molt needs git on the PATH"),
            )
        })?;
    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    } else {
        Ok(None)
    }
}

/// Normalizes a git remote url to an https repository url, returning `None`
/// for the template's own remote (a plain `git clone` of `fuz_template`
/// keeps origin pointed at the template — deriving that would be wrong).
pub fn normalize_remote_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.contains("fuzdev/fuz_template") {
        return None;
    }
    let https = url.strip_prefix("git@").map_or_else(
        || url.to_owned(),
        |rest| format!("https://{}", rest.replacen(':', "/", 1)),
    );
    let trimmed = https.strip_suffix(".git").unwrap_or(&https);
    trimmed.starts_with("https://").then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_url_normalization() {
        assert_eq!(
            normalize_remote_url("git@github.com:you/app.git\n"),
            Some("https://github.com/you/app".to_owned())
        );
        assert_eq!(
            normalize_remote_url("https://github.com/you/app.git"),
            Some("https://github.com/you/app".to_owned())
        );
        assert_eq!(
            normalize_remote_url("https://github.com/you/app"),
            Some("https://github.com/you/app".to_owned())
        );
        assert_eq!(
            normalize_remote_url("git@github.com:fuzdev/fuz_template.git"),
            None
        );
        assert_eq!(
            normalize_remote_url("https://github.com/fuzdev/fuz_template"),
            None
        );
        assert_eq!(normalize_remote_url("/local/path"), None);
    }
}
