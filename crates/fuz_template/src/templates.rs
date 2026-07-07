//! Embedded output templates, substituted with `__PLACEHOLDER__` tokens.

pub const PAGE_SVELTE: &str = include_str!("../templates/page.svelte.in");
pub const README_MD: &str = include_str!("../templates/README.md.in");
pub const CLAUDE_MD: &str = include_str!("../templates/CLAUDE.md.in");
pub const README_RUST_SECTION: &str = include_str!("../templates/readme_rust_section.md.in");
pub const CLAUDE_RUST_SECTION: &str = include_str!("../templates/claude_rust_section.md.in");
pub const WORKSPACE_CARGO_TOML: &str = include_str!("../templates/workspace_cargo.toml.in");
pub const FUNDING_YML: &str = include_str!("../templates/funding.yml.in");

/// The starter page's docs link, substituted only when docs are kept.
pub const PAGE_DOCS_LINK: &str = " \u{b7} <a href={resolve('/docs')}>docs</a>";
/// The generated CLAUDE.md's docs bullet, substituted only when docs are kept.
pub const CLAUDE_DOCS_BULLET: &str = "- `src/routes/docs/` \u{2014} documentation pages with auto-generated API docs from\n  the `svelte-docinfo` Vite plugin\n";

/// Substitutes `__PLACEHOLDER__` tokens in an embedded template.
///
/// Single-pass over the template: inserted values are never re-scanned, so
/// user-provided text (a description containing `__RUST_SECTION__`, say)
/// can't corrupt the output.
pub fn render(template: &str, substitutions: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while !rest.is_empty() {
        let earliest = substitutions
            .iter()
            .filter_map(|(token, value)| rest.find(token).map(|idx| (idx, *token, *value)))
            .min_by_key(|(idx, ..)| *idx);
        let Some((idx, token, value)) = earliest else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..idx]);
        out.push_str(value);
        rest = &rest[idx + token.len()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_substitutes_all_occurrences() {
        assert_eq!(
            render("hi __NAME__, __NAME__!", &[("__NAME__", "sam")]),
            "hi sam, sam!"
        );
    }

    #[test]
    fn render_does_not_rescan_inserted_values() {
        // a value containing another token must pass through literally
        assert_eq!(
            render("a __X__ b __Y__", &[("__X__", "__Y__"), ("__Y__", "z")]),
            "a __Y__ b z"
        );
    }

    #[test]
    fn no_template_placeholders_leak() {
        // Removing every known placeholder must leave no `__` behind, so a
        // typo'd token in a template fails here instead of leaking into output.
        let known = [
            "__NAME__",
            "__NPM_NAME__",
            "__DESCRIPTION_BLOCK__",
            "__RUST_SECTION__",
            "__DOCS_LINK__",
            "__DOCS_BULLET__",
            "__MEMBERS__",
            "__LICENSE__",
        ];
        for template in [
            PAGE_SVELTE,
            README_MD,
            CLAUDE_MD,
            README_RUST_SECTION,
            CLAUDE_RUST_SECTION,
            WORKSPACE_CARGO_TOML,
            FUNDING_YML,
        ] {
            let mut stripped = template.to_owned();
            for token in known {
                stripped = stripped.replace(token, "");
            }
            assert!(
                !stripped.contains("__"),
                "unknown __ token in template: {:?}",
                &stripped[stripped.find("__").unwrap_or(0)..]
            );
        }
    }
}
