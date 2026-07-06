//! Embedded output templates, substituted with `__PLACEHOLDER__` tokens.

pub const PAGE_SVELTE: &str = include_str!("../templates/page.svelte.in");
pub const README_MD: &str = include_str!("../templates/README.md.in");
pub const CLAUDE_MD: &str = include_str!("../templates/CLAUDE.md.in");
pub const README_RUST_SECTION: &str = include_str!("../templates/readme_rust_section.md.in");
pub const CLAUDE_RUST_SECTION: &str = include_str!("../templates/claude_rust_section.md.in");
pub const WORKSPACE_CARGO_TOML: &str = include_str!("../templates/workspace_cargo.toml.in");

/// The starter page's docs link, substituted only when docs are kept.
pub const PAGE_DOCS_LINK: &str = " \u{b7} <a href={resolve('/docs')}>docs</a>";
/// The generated CLAUDE.md's docs bullet, substituted only when docs are kept.
pub const CLAUDE_DOCS_BULLET: &str = "- `src/routes/docs/` \u{2014} documentation pages with auto-generated API docs from\n  the `svelte-docinfo` Vite plugin\n";

/// Substitutes `__PLACEHOLDER__` tokens in an embedded template.
pub fn render(template: &str, substitutions: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (token, value) in substitutions {
        out = out.replace(token, value);
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
        ];
        for template in [
            PAGE_SVELTE,
            README_MD,
            CLAUDE_MD,
            README_RUST_SECTION,
            CLAUDE_RUST_SECTION,
            WORKSPACE_CARGO_TOML,
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
