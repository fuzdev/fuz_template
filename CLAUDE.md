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
- fuz_css (@fuzdev/fuz_css) - semantic-first CSS framework and design system
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
npm run molt # or `cargo molt`
```

**Files molt customizes (or do it by hand):**

- `package.json` - name, description, homepage, repository, glyph/logo fields
- `src/routes/+layout.svelte` - `<title>`, template logo
- `src/routes/+page.svelte` - replace demo content
- `src/routes/about/+page.svelte` - heading
- `src/lib/Mreows.svelte` and `src/lib/Positioned.svelte` - deleted (demo components)
- `LICENSE` and the `license` fields - deleted (the MIT license is the
  template's; your project chooses its own)
- `static/CNAME` - update or delete for your domain
- `.github/FUNDING.yml` and `.github/ISSUE_TEMPLATE/` - update or delete
- `README.md` and `CLAUDE.md` - regenerate for the new project

Not customized: `static/logo.svg` and `static/favicon.png` keep the template's
spider (molt prints a reminder), and `package-lock.json` keeps the old name
until you run `npm i`.

## molt (self-eject CLI)

molt is a one-shot wizard that personalizes the clone and then deletes
itself (like create-react-app's eject, or a spider shedding its skin). It
ships as **twin implementations** at full behavior parity — same flags, same
wizard, byte-identical output — so ejecting never requires a toolchain you
don't have: `src/lib/molt.ts` (run via `npm run molt`) and the `molt` crate
(run via the cargo alias in `.cargo/config.toml`).

```bash
npm run molt           # interactive wizard: prompts, prints the plan, confirms
npm run molt -- check  # verify molt's anchors still match the template
npm run molt -- --help # all flags, for non-interactive use
cargo molt             # the Rust twin: same flags, same behavior
cargo molt check
```

Key behaviors:

- **Requires a git repo with a clean tree** (exit 2 otherwise; `--force`
  overrides dirty, nothing overrides no-git); an applied plan is undone with
  `git reset --hard && git clean -fd` (the tree was clean, so `git clean`
  removes only files molt created).
  Applying to a dirty tree (only reachable via `--force`) always demands the
  dirty-specific in-the-moment confirmation — on the `--wetrun` path and the
  wizard path alike; `--wetrun` alone never skips it, and without a terminal
  the dirty apply is refused (exit 2). A terminal always confirms before
  applying, even with `--wetrun` — the only ungated write path is `--wetrun`
  on a clean tree without a terminal.
- **Plan-then-apply**: every file edit is anchored on exact current content;
  one unmatched anchor aborts before any write, and anchors re-verify after
  a confirm prompt, immediately before writing. Non-interactive runs write
  nothing without `--wetrun`.
- **Feature registry**: every optional feature is a keep/strip choice —
  `rust` (the whole workspace: `Cargo.toml`, `Cargo.lock`, `crates/`,
  `rust-toolchain.toml`, `clippy.toml`, and the `rust` CI job), `cli` (the
  starter crate `crates/app_cli`, renamed to `crates/{name}` with every
  `app_cli` occurrence substituted), `docs` (`src/routes/docs/` +
  `src/routes/library.ts` + the starter page's docs link + the
  svelte-docinfo tooling: devDependency, Vite plugin, ambient types), and
  `github-extras` (FUNDING.yml + issue templates; the only default-strip —
  kept copies are personalized with funding placeholders and discussion
  links pointed at your repo url).
  One prompt each in the wizard — except `cli`, which rides with the `rust`
  prompt while it's `rust`'s only member feature (a kept workspace needs a
  member crate, so there is no separate decision; the `member_of` registry
  field generalizes this) — or `--keep`/`--strip` id lists (comma-separated
  or repeated). Stripping `rust` cascades to `cli`; stripping `cli` while
  keeping `rust` is rejected (cargo can't load an empty workspace), and a
  prompt whose answer explicit flags already force (`--keep cli` forces the
  workspace, `--strip cli` forces stripping rust) is skipped with a note.
  `.cargo/` (which holds only the `cargo molt` alias) and the MIT `LICENSE`
  + `license` fields are deleted unconditionally — the license is fuz.dev's,
  not the new project's, so keeping it is never right (same reasoning as the
  personalized github-extras). The registry lives in
  `crates/molt/src/features.rs` and its TS twin in `molt.ts` — new features
  are one entry + a plan fragment in each twin (the check sample configs
  derive from the registry).
- **Identity fields**: name (required), npm name, description, domain,
  repo url — derived from the git origin when it isn't the template's.
- **Self-verifying**: `cargo molt check` / `npm run molt -- check` and both
  twins' tests verify every anchor against the working tree — and that the
  workspace manifest template stays byte-identical to the live root
  `Cargo.toml` apart from the members and license lines — so a template edit
  that would break ejection fails `cargo test` and `gro test` (and CI) at
  the same commit. When you edit an anchored file, update the anchors in
  `crates/molt/src/anchors.rs` **and** `molt.ts` (and the shared output
  templates in `crates/molt/templates/`) in the same change.
- **Twin discipline**: the twins share identifier-level naming
  (`build_plan`/`verify`/`apply`/`apply_gate`/`FEATURES`…) and `molt.ts`'s
  sections mirror the crate's module seams. The output templates are
  single-sourced in `crates/molt/templates/` (compiled into the Rust binary
  via `include_str!`, read at runtime by the TS twin). Each twin's plan
  deletes both implementations — the crate, `molt.ts`, `molt.test.ts`, and
  the `molt` npm script — so a molted project ships neither.

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
(usage, preconditions, anchor drift), `1` everything else. Arg parsing uses
argh with an explicit `from_args` so usage errors exit `2`.

## Architecture

### Directory structure

```
Cargo.toml                 # Rust workspace (lints, profile, deps)
crates/
├── app_cli/               # starter CLI crate — molt renames it to yours
└── molt/                  # molt's Rust twin — the self-eject CLI (deletes itself)
    ├── src/               # plan/verify/apply, wizard, features, anchors
    └── templates/         # output templates (*.in), shared by both twins
src/
├── app.html               # HTML entry with theme detection
├── lib/                   # your library code
│   ├── Mreows.svelte      # example component (replace me)
│   ├── Positioned.svelte  # example component (replace me)
│   └── molt.ts            # molt's TS twin — `npm run molt` (deletes itself)
├── test/
│   ├── example.test.ts    # test file example
│   └── molt.test.ts       # molt TS twin checks: anchors + apply samples
└── routes/
    ├── +layout.svelte     # root layout with fuz_css imports
    ├── +layout.ts         # prerender: true, ssr: true
    ├── +page.svelte       # home page
    ├── style.css          # custom global styles
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
- `svelte.config.js` enables runes mode and includes a commented-out example
  CSP config using `create_csp_directives()` from fuz_ui
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
- **Minimal test coverage** - One example test file; the rest of the suite
  covers molt, which deletes itself on eject
- **Static only** - No dynamic server-side content

## Project standards

- TypeScript strict mode
- Svelte 5 with runes API
- Prettier with tabs, 100 char width
- Node >= 24.14
- Rust pinned via `rust-toolchain.toml` (edition 2024)
- Private package (not published to npm)

## Related projects

- [`fuz_css`](../fuz_css/CLAUDE.md) - semantic-first CSS framework
- [`fuz_ui`](../fuz_ui/CLAUDE.md) - UI components and docs system
- [`fuz_util`](../fuz_util/CLAUDE.md) - utility functions
- [`fuz_blog`](../fuz_blog/CLAUDE.md) - extends template with blog features
