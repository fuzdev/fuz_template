# @fuzdev/fuz_template ❄

[<img src="/static/logo.svg" alt="a friendly pixelated spider facing you" align="right" width="192" height="192">](https://template.fuz.dev/)

> a web app template with TypeScript + SvelteKit + optional Rust for the fuz-stack

[**template.fuz.dev**](https://template.fuz.dev/)

## contents

- [SvelteKit](https://github.com/sveltejs/kit) with [Svelte](https://github.com/sveltejs/svelte) and
  [Vite](https://github.com/vitejs/vite) and optional [Rust](https://rust-lang.org/)
- [fuz_css](https://github.com/fuzdev/fuz_css): semantic-first CSS framework and design system
- [fuz_ui](https://github.com/fuzdev/fuz_ui):
  - Svelte UI library - [ui.fuz.dev](https://ui.fuz.dev/)
  - optional: `npm uninstall @fuzdev/fuz_ui` and delete the imports to remove it
- [Gro](https://github.com/fuzdev/gro):
  - extends [SvelteKit](https://github.com/sveltejs/kit) and
    [Vite](https://github.com/vitejs/vite)
  - integrated [TypeScript](https://github.com/microsoft/TypeScript)
    with [Svelte](https://github.com/sveltejs/svelte) and
    [svelte-check](https://github.com/sveltejs/language-tools/tree/master/packages/svelte-check)
  - testing with [Vitest](https://github.com/vitest-dev/vitest)
  - formatting with [tsv](https://github.com/fuzdev/tsv)
  - linting with [ESLint](https://github.com/eslint/eslint)
    and [`@ryanatkn/eslint-config`](https://github.com/ryanatkn/eslint-config)
  - also has [a task system](https://github.com/fuzdev/gro/blob/main/src/docs/task.md)
    with [builtins](https://github.com/fuzdev/gro/blob/main/src/docs/tasks.md),
    [codegen](https://github.com/fuzdev/gro/blob/main/src/docs/gen.md),
    and [other things](https://github.com/fuzdev/gro/tree/main/src/lib/docs)
- optional [utilities library `@fuzdev/fuz_util`](https://github.com/fuzdev/fuz_util)

## usage

This project uses [SvelteKit](https://kit.svelte.dev/) with the static adapter
and [Vite](https://vitejs.dev/),
so the normal commands like `vite dev` work as expected.
It also uses [Gro](https://github.com/fuzdev/gro)
for tasks like deploying and more.

If you're logged into GitHub, click "Use this template" above, or clone directly:

```bash
git clone https://github.com/fuzdev/fuz_template.git cooltoy
cd cooltoy
npm i
# then
vite dev
# or
npm run dev
# or
gro dev # npm i -g @fuzdev/gro
gro sync # called by `gro dev`, refreshes generated files and calls `svelte-kit sync`
```

> learn more about [SvelteKit](https://github.com/sveltejs/kit),
> [Vite](https://github.com/vitejs/vite), [Gro](https://github.com/fuzdev/gro),
> and [fuz_ui](https://github.com/fuzdev/fuz_ui)

> [Windows will not be supported](https://github.com/fuzdev/fuz_template/issues/4) because
> it depends on sh, [WSL](https://docs.microsoft.com/en-us/windows/wsl/about) is recommended

The template includes
[`@sveltejs/adapter-static`](https://github.com/sveltejs/kit/tree/master/packages/adapter-static)
so it can [deploy](https://github.com/fuzdev/gro/blob/main/src/docs/deploy.md)
with no further configuration.
To learn how to swap it out for another deployment target, see
[the SvelteKit adapter docs](https://svelte.dev/docs/kit/adapters).
SvelteKit's config lives in [`vite.config.ts`](vite.config.ts),
passed to the `sveltekit({...})` plugin.

To make it your own, run the molt wizard (it requires a clean git tree):

```bash
npm run molt # or `cargo molt` — twin implementations, identical results
```

It renames the project, strips the demo components, and deletes itself — see
[molt](#molt) below.

To do the same by hand, change `@fuzdev/fuz_template`
and `template.fuz.dev` to your project name in the following files:

- [`package.json`](package.json) — also remove or replace the `glyph`,
  `logo`, and `logo_alt` fields
- [`src/routes/+layout.svelte`](src/routes/+layout.svelte)
- [`src/routes/+page.svelte`](src/routes/+page.svelte)
- [`src/routes/about/+page.svelte`](src/routes/about/+page.svelte)
- update or delete [`static/CNAME`](static/CNAME),
  ./.github/FUNDING.yml, and ./.github/ISSUE_TEMPLATE/

To remove the Rust workspace, delete `Cargo.toml`, `Cargo.lock`, `crates/`,
`.cargo/`, `rust-toolchain.toml`, `clippy.toml`, and the `rust` job in
[`.github/workflows/check.yml`](.github/workflows/check.yml).

To remove the docs system, delete `src/routes/docs/` and
[`src/routes/library.ts`](src/routes/library.ts), remove the docs link in
[`src/routes/+page.svelte`](src/routes/+page.svelte), and drop the
`svelte-docinfo` devDependency along with its wiring in
[`vite.config.ts`](vite.config.ts) and [`src/app.d.ts`](src/app.d.ts).

Then run `npm i` to update `package-lock.json`.

The template is MIT-licensed ([`LICENSE`](LICENSE), plus `license` fields in
`package.json` and `Cargo.toml`), copyright fuz.dev — to keep MIT for your
project, replace the copyright holder with your own; or swap in
[another license](https://choosealicense.com/).

## molt

molt is a wizard that transforms this clone into your own project, then
deletes itself. It ships as twin implementations at full behavior parity —
same flags, same prompts, byte-identical results — so either toolchain
suffices: `src/lib/molt.ts` runs on Node, and the `molt` crate comes with
the template's Rust workspace.

```bash
npm run molt          # interactive wizard: prompts, prints the full plan, confirms
npm run molt -- check # verify molt's file anchors still match the template
cargo molt            # the Rust twin — same flags and behavior
cargo molt check
```

The plan, from the prompts or the equivalent flags:

- renames the project everywhere (`package.json`, page titles, headings)
- updates or removes `static/CNAME`, `homepage`, and `repository`
- deletes the demo components and writes a minimal starting page (plus a
  starter `src/lib/example.ts` when docs are kept — the API docs need a
  module to document)
- regenerates `README.md` and `CLAUDE.md` for the new project
- deletes the template's MIT `LICENSE` and `license` fields — the new project
  chooses its own license
- keeps or strips each optional feature, by prompt or by `--keep`/`--strip`
  lists (e.g. `--strip rust` or `--keep github-extras --strip docs`):
  - `rust` — the whole workspace
  - `cli` — the starter crate at [`crates/app_cli`](crates/app_cli), renamed
    to the project name; it rides with `rust`, and stripping `cli` alone is
    rejected because a kept workspace needs a crate
  - `docs` — the docs system; keeping it needs a repository url (the API
    docs link to source) from `--repo` or a git origin that isn't the
    template's, and the wizard asks when there's none
  - `github-extras` — funding + issue templates, personalized when kept;
    stripped by default
- deletes both of its own implementations (the crate, the script, and its tests)

`--repo` takes an https, http, or ssh git url, normalizes it to its web url
with any credentials stripped, and refuses anything else with exit 2.

It refuses to run without a clean git tree, so an applied plan can be undone
with `git reset --hard && git clean -fd` (the tree was clean, so `git clean`
removes only files molt created). `--force` lets a dirty tree through for
_planning_, but applying to a dirty tree always requires an interactive
confirmation — without a terminal it refuses (exit 2), since there is no
clean undo point. Flags replace the prompts for non-interactive use
(`npm run molt -- --help` or `cargo molt --help`); in that mode nothing is
written without `--wetrun`. A terminal always confirms before applying, so
the only ungated write path is `--wetrun` on a clean tree without a terminal.

## build

```bash
npm run build
# or
gro build
```

See [Gro's build docs](https://github.com/fuzdev/gro/blob/main/src/docs/build.md) for more.

## test

```bash
npm test
# or
gro test
gro test filepattern1 filepatternB
gro test -- --forwarded-args 'to vitest'
```

See [Vitest](https://github.com/vitest-dev/vitest),
[`src/test/example.test.ts`](src/test/example.test.ts),
and [Gro's test docs](https://github.com/fuzdev/gro/blob/main/src/docs/test.md) for more.

## deploy

[Deploy](https://github.com/fuzdev/gro/blob/main/src/docs/deploy.md)
(build, commit, and push) to the `deploy` branch, e.g. for GitHub Pages:

```bash
npm run deploy
# or
gro deploy
```

## credits 🐢<sub>🐢</sub><sub><sub>🐢</sub></sub>

[Svelte](https://github.com/sveltejs/svelte) ∙
[SvelteKit](https://github.com/sveltejs/kit) ∙
[Vite](https://github.com/vitejs/vite) ∙
[esbuild](https://github.com/evanw/esbuild) ∙
[Vitest](https://github.com/vitest-dev/vitest) ∙
[TypeScript](https://github.com/microsoft/TypeScript) ∙
[ESLint](https://github.com/eslint/eslint) ∙
[tsv](https://github.com/fuzdev/tsv) ∙
[fuz_css](https://github.com/fuzdev/fuz_css) ∙
[Fuz](https://github.com/fuzdev/fuz_ui) ∙
[Gro](https://github.com/fuzdev/gro) ∙
[@fuzdev/fuz_util](https://github.com/fuzdev/fuz_util) ∙
[Zod](https://github.com/colinhacks/zod) ∙
& [more](package.json)

## [🐦](https://wikipedia.org/wiki/Free_and_open-source_software)
