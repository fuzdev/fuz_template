use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::error::CliError;

/// Fully-resolved molt choices, assembled from flags and wizard answers.
/// `kept` holds the feature ids (from `features::FEATURES`) to keep.
#[derive(Debug)]
pub struct MoltConfig {
    pub name: String,
    pub npm_name: String,
    pub description: String,
    pub domain: Option<String>,
    pub repo_url: Option<String>,
    pub kept: BTreeSet<&'static str>,
}

impl MoltConfig {
    pub fn keeps(&self, feature_id: &str) -> bool {
        self.kept.contains(feature_id)
    }
}

/// Names cargo refuses as package names: Rust keywords (strict + reserved,
/// including 2024's `gen`) plus `test`, which conflicts with the built-in
/// test library. The name becomes the starter crate's name, so a keyword
/// would leave the molted workspace unable to build.
const RESERVED_CRATE_NAMES: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "test", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Validates a project name: `snake_case`, usable as a crate name.
pub fn validate_name(name: &str) -> Result<(), CliError> {
    let valid_first = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    let valid_rest = name
        .chars()
        .skip(1)
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !valid_first || !valid_rest {
        return Err(CliError::Usage(format!(
            "invalid name {name:?}: use snake_case starting with a letter (e.g. my_app)"
        )));
    }
    if RESERVED_CRATE_NAMES.contains(&name) {
        return Err(CliError::Usage(format!(
            "name {name:?} can't be used as a crate name (Rust keyword or built-in) — pick another"
        )));
    }
    if matches!(name, "fuz_template" | "app_cli" | "xtask") {
        return Err(CliError::Usage(format!(
            "name {name:?} is reserved — pick your own project name"
        )));
    }
    Ok(())
}

/// Validates a project description: a single line, no control characters
/// (it lands in `package.json`, TOML, and markdown blockquotes).
pub fn validate_description(description: &str) -> Result<(), CliError> {
    if description.chars().any(char::is_control) {
        return Err(CliError::Usage(
            "description must be a single line without control characters".to_owned(),
        ));
    }
    Ok(())
}

/// Validates an npm package name loosely (scoped names like `@you/name` allowed).
pub fn validate_npm_name(name: &str) -> Result<(), CliError> {
    let valid = !name.is_empty()
        && name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.' | '/' | '@')
        });
    if valid {
        Ok(())
    } else {
        Err(CliError::Usage(format!(
            "invalid npm package name {name:?}"
        )))
    }
}

/// Validates a bare domain like `example.com` (no scheme, no path).
pub fn validate_domain(domain: &str) -> Result<(), CliError> {
    let valid = !domain.is_empty()
        && domain.contains('.')
        && domain
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'));
    if valid {
        Ok(())
    } else {
        Err(CliError::Usage(format!(
            "invalid domain {domain:?}: expected a bare domain like example.com"
        )))
    }
}

/// Escapes a string for embedding in a JSON string literal (also valid for
/// TOML basic strings, which share the `\"`/`\\`/`\n` escapes).
pub fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation() {
        assert!(validate_name("my_app").is_ok());
        assert!(validate_name("app2").is_ok());
        assert!(validate_name("My_App").is_err());
        assert!(validate_name("2app").is_err());
        assert!(validate_name("my-app").is_err());
        assert!(validate_name("").is_err());
        assert!(validate_name("fuz_template").is_err());
        assert!(validate_name("app_cli").is_err());
        // Rust keywords and `test` can't be crate names
        assert!(validate_name("match").is_err());
        assert!(validate_name("loop").is_err());
        assert!(validate_name("test").is_err());
        assert!(validate_name("gen").is_err());
        assert!(validate_name("matcher").is_ok());
    }

    #[test]
    fn description_validation() {
        assert!(validate_description("").is_ok());
        assert!(validate_description("a fine one-liner").is_ok());
        assert!(validate_description("line\nbreak").is_err());
        assert!(validate_description("tab\there").is_err());
    }

    #[test]
    fn npm_name_validation() {
        assert!(validate_npm_name("my_app").is_ok());
        assert!(validate_npm_name("@you/my-app").is_ok());
        assert!(validate_npm_name("").is_err());
        assert!(validate_npm_name("My App").is_err());
    }

    #[test]
    fn domain_validation() {
        assert!(validate_domain("example.com").is_ok());
        assert!(validate_domain("sub.example.co.uk").is_ok());
        assert!(validate_domain("https://example.com").is_err());
        assert!(validate_domain("nodots").is_err());
    }

    #[test]
    fn json_escaping() {
        assert_eq!(json_escape("plain"), "plain");
        assert_eq!(json_escape("a \"b\" c"), "a \\\"b\\\" c");
        assert_eq!(json_escape("line\nbreak"), "line\\nbreak");
    }
}
