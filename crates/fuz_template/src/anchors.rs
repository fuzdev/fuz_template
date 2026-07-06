//! Exact-content anchors into the template's files.
//!
//! molt and the template travel in the same repo at the same commit, so these
//! can be exact strings. `cargo molt check` (and this crate's tests) verify
//! every anchor still matches — when you edit an anchored spot in the
//! template, update the anchor here in the same change.

pub const PACKAGE_JSON_NAME: &str = "  \"name\": \"@fuzdev/fuz_template\",\n";
pub const PACKAGE_JSON_DESCRIPTION: &str = "  \"description\": \"a static web app and Node library template with TypeScript, Svelte, SvelteKit, Vite, esbuild, Gro, and Fuz\",\n";
pub const PACKAGE_JSON_GLYPH: &str = "  \"glyph\": \"\u{2744}\",\n";
pub const PACKAGE_JSON_LOGO: &str = "  \"logo\": \"logo.svg\",\n";
pub const PACKAGE_JSON_LOGO_ALT: &str =
    "  \"logo_alt\": \"a friendly pixelated spider facing you\",\n";
pub const PACKAGE_JSON_HOMEPAGE: &str = "  \"homepage\": \"https://template.fuz.dev/\",\n";
pub const PACKAGE_JSON_REPOSITORY: &str =
    "  \"repository\": \"https://github.com/fuzdev/fuz_template\",\n";

pub const LAYOUT_LOGO_IMPORT: &str =
    "\timport {logo_fuz_template} from '@fuzdev/fuz_ui/logos.ts';\n";
pub const LAYOUT_SITE_STATE: &str = "\t// `glyph` and `repo_url` derive from `pkg_json`; `icon` stays explicit (structured `SvgData`).\n\tsite_context.set(new SiteState({icon: logo_fuz_template, pkg_json}));";
pub const LAYOUT_SITE_STATE_REPLACEMENT: &str = "\t// `glyph` and `repo_url` derive from `pkg_json`.\n\tsite_context.set(new SiteState({pkg_json}));";
pub const LAYOUT_TITLE: &str = "<title>@fuzdev/fuz_template</title>";

pub const PAGE_MREOWS_IMPORT: &str = "import Mreows, {mreow_items} from '$lib/Mreows.svelte';";
pub const H1_FUZ_TEMPLATE: &str = "<h1 class=\"mt_xl2\">fuz_template</h1>";

// the docs system's tooling, stripped with the `docs` feature
pub const PACKAGE_JSON_SVELTE_DOCINFO: &str = "    \"svelte-docinfo\": \"^0.5.3\",\n";
pub const VITE_DOCINFO_IMPORT: &str = "import svelte_docinfo from 'svelte-docinfo/vite.js';\n";
pub const VITE_DOCINFO_PLUGIN: &str = "svelte_docinfo(), ";
pub const APP_D_TS_DOCINFO: &str = "// Registers ambient types for the `virtual:svelte-docinfo` module (Vite plugin).\n// eslint-disable-next-line @typescript-eslint/triple-slash-reference\n/// <reference types=\"svelte-docinfo/virtual-svelte-docinfo.js\" />\n";

// the github extras, personalized when kept
pub const FUNDING_GITHUB: &str = "github: ryanatkn";
/// The template's repo url as it appears in the issue-template discussion
/// links — replaced with the molted project's repo url when derivable.
pub const TEMPLATE_REPO_URL: &str = "https://github.com/fuzdev/fuz_template";

pub const README_H1: &str = "# @fuzdev/fuz_template \u{2744}";
pub const CLAUDE_H1: &str = "# fuz_template\n";
pub const CNAME_CONTENT: &str = "template.fuz.dev";
pub const WORKSPACE_MEMBERS: &str = "members = [\"crates/app_cli\", \"crates/fuz_template\"]";

/// The starter CLI crate's name — molt renames every occurrence (and the
/// crate directory) to the chosen project name.
pub const APP_CLI_TOKEN: &str = "app_cli";
/// The starter CLI crate's placeholder description line.
pub const APP_CLI_DESCRIPTION: &str = "description = \"a CLI scaffolded by fuz_template's molt\"\n";

/// The `rust` job appended to `.github/workflows/check.yml` — kept here as an
/// exact-match anchor so stripping the `rust` feature can remove it.
pub const CI_RUST_JOB: &str = r"
  rust:
    # molt anchors this job (crates/fuz_template/src/anchors.rs) so stripping
    # the rust feature can remove it — update the anchor when editing.
    runs-on: ubuntu-latest
    timeout-minutes: 15

    steps:
      - uses: actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10 # v6.0.3
        with:
          persist-credentials: false
      # rustup auto-installs the toolchain pinned in rust-toolchain.toml
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
";
