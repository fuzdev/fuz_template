use std::path::Path;
use std::process::Command;

use crate::error::CliError;

/// Repo-locating git variables that git hooks set — cleared for every git
/// call so git always resolves the repo from the given root.
const INHERITED_GIT_ENV: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

/// Runs a git command at `root`, returning its stdout on success and `None`
/// on a nonzero exit (e.g. no `origin` remote configured). Clears the
/// `INHERITED_GIT_ENV` variables so git always resolves the repo from `root`.
pub fn output(root: &Path, args: &[&str]) -> Result<Option<String>, CliError> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    for var in INHERITED_GIT_ENV {
        command.env_remove(var);
    }
    let output = command.output().map_err(|source| {
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

/// Trims whitespace and byte-order marks from a url, matching the TS twin's
/// `trim_url` (Rust's `trim` keeps U+FEFF, which JS's strips).
pub fn trim_url(url: &str) -> &str {
    url.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// Converts a repository url in any common git form (https or http,
/// scp-style `git@host:`, or `ssh://git@host/`) to its web url — the ssh
/// forms become https without their ssh port, userinfo (a user or token)
/// is dropped, the scheme is lowercased, an empty port is dropped, a
/// `?query` or `#fragment` is dropped from the `ssh://` and http(s) forms
/// (the scp form keeps `?`/`#` as literal path characters), and a trailing
/// `/` or `.git` is dropped — returning `None` for anything else (a local
/// path, a bare `owner/repo`, an empty host).
pub fn to_repo_url(url: &str) -> Option<String> {
    let url = trim_url(url);
    let web = if let Some((scheme, rest)) = url.split_once("://") {
        match scheme.to_ascii_lowercase().as_str() {
            "ssh" => {
                let (authority, path) = split_authority(rest.strip_prefix("git@")?);
                // the ssh daemon's port (even an empty one) means nothing to
                // the web host
                let host = match authority.rsplit_once(':') {
                    Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
                    _ => authority,
                };
                if host.is_empty() || host.starts_with(':') {
                    return None;
                }
                format!("https://{host}{path}")
            }
            scheme @ ("https" | "http") => {
                let (authority, path) = split_authority(rest);
                // userinfo (a user, or a token) never belongs in a web url
                let host = authority
                    .rsplit_once('@')
                    .map_or(authority, |(_, host)| host);
                // an empty port adds nothing; a host that's only a port is none
                let host = host.strip_suffix(':').unwrap_or(host);
                if host.is_empty() || host.starts_with(':') {
                    return None;
                }
                format!("{scheme}://{host}{path}")
            }
            _ => return None,
        }
    } else {
        let rest = url.strip_prefix("git@")?;
        if rest.find([':', '/']).unwrap_or(rest.len()) == 0 {
            return None;
        }
        format!("https://{}", rest.replacen(':', "/", 1))
    };
    let web = web.strip_suffix('/').unwrap_or(&web);
    let web = web.strip_suffix(".git").unwrap_or(web);
    let (_, host_and_path) = web.split_once("://")?;
    (!host_and_path.is_empty()).then(|| web.to_owned())
}

/// Splits a url's remainder after `scheme://` into its authority, which ends
/// at the first `/`, `?`, or `#`, and its path, dropping any `?query` or
/// `#fragment` (meaningless for a repository url).
fn split_authority(rest: &str) -> (&str, &str) {
    let (authority, tail) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    (
        authority,
        &tail[..tail.find(['?', '#']).unwrap_or(tail.len())],
    )
}

/// Normalizes the git origin url with `to_repo_url`, returning `None` for
/// the template's own remote (a plain `git clone` of `fuz_template` keeps
/// origin pointed at the template — deriving that would be wrong). GitHub
/// owner and repo names are case-insensitive, so the check is too.
pub fn normalize_remote_url(url: &str) -> Option<String> {
    if url.to_ascii_lowercase().contains("fuzdev/fuz_template") {
        return None;
    }
    to_repo_url(url)
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
            normalize_remote_url("ssh://git@github.com/you/app.git"),
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
        assert_eq!(
            normalize_remote_url("https://github.com/FuzDev/Fuz_Template.git"),
            None
        );
        assert_eq!(normalize_remote_url("/local/path"), None);
        // a user's own http origin is kept as-is rather than dropped
        assert_eq!(
            normalize_remote_url("http://git.example.com/you/app.git"),
            Some("http://git.example.com/you/app".to_owned())
        );
        // a CI or embedded-token origin never leaks its credentials
        assert_eq!(
            normalize_remote_url("https://x-access-token:ghs_secret@github.com/you/app.git\n"),
            Some("https://github.com/you/app".to_owned())
        );
    }

    #[test]
    fn repo_url_conversion() {
        for (input, expected) in [
            (
                "git@github.com:me/demo_app.git",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "ssh://git@github.com/me/demo_app.git",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "https://github.com/me/demo_app/",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "http://github.com/me/demo_app",
                Some("http://github.com/me/demo_app"),
            ),
            // explicit input skips the template check
            (
                "https://github.com/fuzdev/fuz_template.git",
                Some("https://github.com/fuzdev/fuz_template"),
            ),
            // credentials and user names are dropped
            (
                "https://user:token@github.com/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "https://user@github.com/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            ("https://user:token@", None),
            // an ssh port is dropped; an https port is kept
            (
                "ssh://git@git.example.com:2222/me/demo_app.git",
                Some("https://git.example.com/me/demo_app"),
            ),
            (
                "https://git.example.com:8443/me/demo_app",
                Some("https://git.example.com:8443/me/demo_app"),
            ),
            // schemes are case-insensitive; a byte-order mark is trimmed
            (
                "HTTPS://github.com/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "SSH://git@github.com/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            (
                "\u{feff}https://github.com/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            ("ssh://me@github.com/me/demo_app", None),
            ("ftp://github.com/me/demo_app", None),
            // an empty host is refused, whatever follows it
            ("https://user:token@/me/app", None),
            ("https://@/me/app", None),
            ("https:///me/app", None),
            ("ssh://git@/me/app", None),
            ("git@:me/app", None),
            ("https://:8443/me/app", None),
            ("ssh://git@::22/me/app", None),
            ("ssh://git@:8443:22/me/app", None),
            ("https://user@:443/me/app", None),
            // an empty https port is dropped, like an empty ssh port
            (
                "https://github.com:/me/demo_app",
                Some("https://github.com/me/demo_app"),
            ),
            // the authority ends at `?` or `#` too, and a query or fragment
            // is dropped
            (
                "https://github.com?x@evil.com/me/app",
                Some("https://github.com"),
            ),
            (
                "https://github.com#@evil.com/me/app",
                Some("https://github.com"),
            ),
            (
                "https://github.com/me/demo_app?tab=readme#top",
                Some("https://github.com/me/demo_app"),
            ),
            // an empty ssh port is dropped too
            (
                "ssh://git@git.example.com:/me/demo_app",
                Some("https://git.example.com/me/demo_app"),
            ),
            ("me/demo_app", None),
            ("https://", None),
            ("", None),
        ] {
            assert_eq!(to_repo_url(input).as_deref(), expected, "{input:?}");
        }
    }
}
