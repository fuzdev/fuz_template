# fuz_template

> SvelteKit starter template with full fuz stack integration

fuz_template (`@fuzdev/fuz_template`) is a production-ready starter template for
building static web applications with the fuz stack. Clone it to start new
projects with TypeScript, Svelte 5, SvelteKit, and the complete fuz ecosystem
pre-configured.

For coding conventions, see Skill(fuz-stack).

## Committing

`git add` and `git commit` are denied by `.claude/settings.local.json` in
this repo — make the edits and stop, the user commits.

## Gro commands

```bash
gro check     # typecheck, test, lint, format check (run before committing)
gro typecheck # typecheck only (faster iteration)
gro test      # run tests with vitest
gro build     # build for production (static adapter)
gro deploy    # build, commit, and push to deploy branch
gro sync      # regenerate files and run svelte-kit sync
```

IMPORTANT for AI agents: Do NOT run `gro dev` - the developer will manage the
dev server.

## Key dependencies

- Svelte 5 - component framework with runes
- SvelteKit - application framework with static adapter
- Vite - build tool
- fuz_css (@fuzdev/fuz_css) - CSS framework and design system
- fuz_ui (@fuzdev/fuz_ui) - UI components, theming, docs system
- fuz_util (@fuzdev/fuz_util) - utility functions
- fuz_code (@fuzdev/fuz_code) - syntax highlighting
- Gro (@fuzdev/gro) - build system and task runner

## Scope

fuz_template is a **SvelteKit starter template**:

- Pre-configured fuz stack (fuz_css, fuz_ui, fuz_util, fuz_code)
- Dark/light theme with persistence
- Documentation system with API generation
- Static deployment ready (GitHub Pages, Netlify)

### What fuz_template does NOT include

- Authentication or user management
- Database or backend
- Dynamic server-side content
- Production-ready components (demos only)

## Using the template

Use GitHub's "Use this template" button or clone directly:

```bash
git clone https://github.com/fuzdev/fuz_template.git myproject
cd myproject
npm i
```

Then transform it into your own project with molt (see below):

```bash
cargo molt
```

**Files molt customizes (or do it by hand):**

- `package.json` - name, description, homepage, repository, glyph/logo fields
- `src/routes/+layout.svelte` - `<title>`, template logo
- `src/routes/+page.svelte` - replace demo content
- `src/routes/about/+page.svelte` - heading
- `static/CNAME` - update or delete for your domain
- `.github/FUNDING.yml` and `.github/ISSUE_TEMPLATE/` - update or delete
- `README.md` and `CLAUDE.md` - regenerate for the new project

## molt (self-eject CLI)

`crates/fuz_template` is molt — a one-shot wizard that personalizes the
clone and then deletes itself (like create-react-app's eject, or a spider
shedding its skin). Invoked via the cargo alias in `.cargo/config.toml`:

```bash
cargo molt         # interactive wizard: prompts, prints the plan, confirms
cargo molt check   # verify molt's anchors still match the template
cargo molt --help  # all flags, for non-interactive use
```

Key behaviors:

- **Requires a git repo with a clean tree** (exit 2 otherwise; `--force`
  overrides dirty, nothing overrides no-git) so it's always undoable.
  Applying to a dirty tree (only reachable via `--force`) always demands an
  in-the-moment interactive confirmation — `--wetrun` alone never skips it,
  and without a terminal the dirty apply is refused (exit 2). The only
  ungated write path is `--wetrun` on a clean tree, where `git checkout` is
  a full undo.
- **Plan-then-apply**: every file edit is anchored on exact current content;
  one unmatched anchor aborts before any write. Non-interactive runs write
  nothing without `--wetrun`.
- **Feature registry**: every optional feature is a keep/strip choice —
  `rust` (the whole workspace: `Cargo.toml`, `crates/`, `.cargo/`,
  `rust-toolchain.toml`, `clippy.toml`, and the `rust` CI job), `cli` (the
  starter crate `crates/app_cli`, renamed to `crates/{name}` with every
  `app_cli` occurrence substituted), `docs` (`src/routes/docs/` +
  `src/routes/library.ts` + the starter page's docs link), and
  `github-extras` (FUNDING.yml + issue templates; the only default-strip).
  One prompt each in the wizard, or `--keep`/`--strip` id lists
  (comma-separated or repeated). Stripping `rust` cascades to `cli`. The
  registry lives in `crates/fuz_template/src/features.rs` — new features are
  one entry + a plan fragment there.
- **Identity fields**: name (required), npm name, description, domain,
  repo url — derived from the git origin when it isn't the template's.
- **Self-verifying**: `cargo molt check` and the crate's tests verify every
  anchor against the working tree, so a template edit that would break
  ejection fails `cargo test` (and CI) at the same commit. When you edit an
  anchored file, update `crates/fuz_template/src/anchors.rs` (and the
  embedded templates in `crates/fuz_template/templates/`) in the same change.

## Rust workspace

The repo doubles as a Rust workspace following the fuz ecosystem's
conventions: root `Cargo.toml` with the canonical lint block
(`unsafe_code = "forbid"`, clippy pedantic/nursery at warn) and release
profile, `clippy.toml` test allowances, and the toolchain pinned in
`rust-toolchain.toml`. Do not use Gro for the Rust side — run cargo
directly:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

CI runs these in the `rust` job of `.github/workflows/check.yml`. Errors
follow thiserror enums with `.exit_code()`/`.hint()` helpers and
`fn main() -> ExitCode`; exit codes: `0` success, `2` caller-must-fix
(usage, preconditions), `1` everything else. Arg parsing uses argh with an
explicit `from_args` so usage errors exit `2`.

## Architecture

### Directory structure

```
Cargo.toml                 # Rust workspace (lints, profile, deps)
crates/
├── app_cli/               # starter CLI crate — molt renames it to yours
└── fuz_template/          # molt — the self-eject CLI (deletes itself)
    ├── src/               # plan/verify/apply, wizard, features, anchors
    └── templates/         # embedded output templates (*.in)
src/
├── app.html               # HTML entry with theme detection
├── lib/                   # your library code
│   ├── Mreows.svelte      # example component (replace me)
│   └── Positioned.svelte  # example component (replace me)
└── routes/
    ├── +layout.svelte     # root layout with fuz_css imports
    ├── +layout.ts         # prerender: true, ssr: true
    ├── +page.svelte       # home page
    ├── style.css          # custom global styles
    ├── example.test.ts    # test file example
    ├── about/+page.svelte
    └── docs/              # documentation pages
        ├── +layout.svelte # wraps docs in Docs component
        ├── +page.svelte   # docs index
        ├── tomes.ts       # documentation structure
        ├── library/       # library details page
        └── api/           # auto-generated API docs
```

### Example components (replace these)

The template includes demo components to show Svelte 5 patterns:

**Mreows.svelte** - interactive emoji grid demo showing `$props()`,
`$bindable()`, `$state()`, `$derived()`. Marked with "don't use this component".

**Positioned.svelte** - CSS transform utility with Snippet children.

Replace these with your actual components.

### SvelteKit configuration

- `+layout.ts` exports `prerender = true` and `ssr = true` for full static
  generation
- `svelte.config.js` enables runes mode and configures CSP via
  `create_csp_directives()` from fuz_ui
- Uses `@sveltejs/adapter-static` for static output

### Theme detection

`app.html` includes theme detection that runs before render:

1. Reads `localStorage.getItem('fuz:color-scheme')`
2. Falls back to `matchMedia('(prefers-color-scheme:dark)')`
3. Sets class on `<html>` element ('dark' or 'light')

This prevents flash of wrong theme on page load.

### Library metadata

Component library metadata (modules, declarations, props, dependencies) is
provided at runtime by the `svelte-docinfo` Vite plugin via the
`virtual:svelte-docinfo` module. `src/routes/library.ts` combines it with
`package.json` through `library_json_from_modules`, and `docs/+layout.svelte`
sets the `library_context` (only where the docs need it — the root layout sets
just the lighter `site_context`), powering auto-generated API docs at
`/docs/api/`.

### CSS utility classes

The `vite_plugin_fuz_css` Vite plugin (wired in `vite.config.ts`) generates
fuz_css utility classes on demand and exposes them via the `virtual:fuz.css`
module, imported in the root `+layout.svelte`. No generated `fuz.css` file is
committed.

### Documentation system

Uses fuz_ui's tome system:

- `docs/tomes.ts` - defines documentation pages
- `docs/library/` - shows `LibraryDetail` component
- `docs/api/` - auto-generated API docs from `virtual:svelte-docinfo`
- `docs/api/[...module_path]/` - dynamic module documentation

## Context system

Uses contexts from fuz_ui:

- `library_context` - provides `Library` class for docs
- `tomes_context` - provides documentation structure
- Theme context via `ThemeRoot` component wrapper

## Static deployment

Pre-configured for static hosting (GitHub Pages, Netlify, etc.):

- Uses `@sveltejs/adapter-static`
- `static/CNAME` for custom domain
- `static/.nojekyll` for GitHub Pages

Deploy with `gro deploy` (builds and pushes to deploy branch).

## Known limitations

- **Demo components only** - Mreows and Positioned are examples, not for
  production use
- **Minimal test coverage** - Only one example test file included
- **Static only** - No dynamic server-side content
- **Tests colocated** - Tests in routes (`example.test.ts`) rather than
  `src/test/` directory

## Project standards

- TypeScript strict mode
- Svelte 5 with runes API
- Prettier with tabs, 100 char width
- Node >= 24.14
- Rust pinned via `rust-toolchain.toml` (edition 2024)
- Private package (not published to npm)

## Related projects

- [`fuz_css`](../fuz_css/CLAUDE.md) - CSS framework
- [`fuz_ui`](../fuz_ui/CLAUDE.md) - UI components and docs system
- [`fuz_util`](../fuz_util/CLAUDE.md) - utility functions
- [`fuz_blog`](../fuz_blog/CLAUDE.md) - extends template with blog features
