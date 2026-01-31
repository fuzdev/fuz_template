# fuz_template

> SvelteKit starter template with full fuz stack integration

fuz_template (`@fuzdev/fuz_template`) is a production-ready starter template for
building static web applications with the fuz stack. Clone it to start new
projects with TypeScript, Svelte 5, SvelteKit, and the complete fuz ecosystem
pre-configured.

For coding conventions, see [`fuz-stack`](../fuz-stack/CLAUDE.md).

## Gro commands

```bash
gro check     # typecheck, test, lint, format check (run before committing)
gro test      # run tests with vitest
gro gen       # regenerate .gen files (library.json, fuz.css)
gro build     # build for production (static adapter)
gro deploy    # build, commit, and push to deploy branch
gro sync      # regenerate files and run svelte-kit sync
```

IMPORTANT for AI agents: Do NOT run `gro dev` - the developer will manage the
dev server.

## Using the template

Clone with degit:

```bash
npx degit fuzdev/fuz_template myproject
cd myproject
npm i
```

Or use GitHub's "Use this template" button.

**Files to customize:**

- `package.json` - name, version, description, homepage, repository
- `svelte.config.js` - update origin URL
- `src/routes/+layout.svelte` - update `<title>`
- `src/routes/+page.svelte` - replace demo content
- `static/CNAME` - update or delete for your domain
- `.github/FUNDING.yml` - update or delete

## Key dependencies

- Svelte 5 - component framework with runes
- SvelteKit - application framework with static adapter
- Vite - build tool
- fuz_css (@fuzdev/fuz_css) - CSS framework and design system
- fuz_ui (@fuzdev/fuz_ui) - UI components, theming, docs system
- fuz_util (@fuzdev/fuz_util) - utility functions
- fuz_code (@fuzdev/fuz_code) - syntax highlighting
- Gro (@ryanatkn/gro) - build system and task runner

## Architecture

### Directory structure

```
src/
├── app.html               # HTML entry with theme detection
├── lib/                   # your library code
│   ├── Mreows.svelte      # example component (replace me)
│   └── Positioned.svelte  # example component (replace me)
├── test/                  # test files (not co-located)
└── routes/
    ├── +layout.svelte     # root layout with fuz_css imports
    ├── +layout.ts         # prerender: true, ssr: true
    ├── +page.svelte       # home page
    ├── style.css          # custom global styles
    ├── fuz.css            # generated fuz_css styles
    ├── library.gen.ts     # generates library.json
    ├── library.ts         # exports library metadata
    ├── library.json       # generated component metadata
    ├── about/+page.svelte
    └── docs/              # documentation pages
        ├── +layout.svelte # wraps docs in Docs component
        ├── +page.svelte   # docs index
        ├── tomes.ts       # documentation structure
        ├── library/       # library details page
        └── api/           # auto-generated API docs
```

### Example components (replace these)

The template includes demo components to show patterns:

**Mreows.svelte** - interactive emoji grid demo

- Shows Svelte 5 patterns: `$props()`, `$bindable()`, `$state()`, `$derived()`
- Exports types from `<script module>`
- Uses layout calculations and transforms
- Marked with "don't use this component" comment

**Positioned.svelte** - CSS transform utility

- Shows props with Snippet children
- Uses `transform: translate3d()` and `scale3d()`
- Smooth transitions

Replace these with your actual components.

### Root layout setup

The template configures the fuz stack in `+layout.svelte`:

```svelte
<script lang="ts">
  import '@fuzdev/fuz_css/style.css';  // semantic styles
  import '@fuzdev/fuz_css/theme.css';  // design tokens
  import '$routes/fuz.css';             // generated utility classes
  import '$routes/style.css';           // your custom styles

  import Themed from '@fuzdev/fuz_ui/Themed.svelte';
</script>

<Themed>
  {@render children()}
</Themed>
```

### Theme detection

`app.html` includes theme detection that reads from localStorage before render:

```javascript
localStorage.getItem('fuz:color-scheme')
```

This prevents flash of wrong theme on page load.

### Documentation system

The template includes a docs structure using fuz_ui's tome system:

- `docs/tomes.ts` - defines documentation pages
- `docs/library/` - shows `LibraryDetail` component
- `docs/api/` - auto-generated API docs from `library.json`
- `docs/api/[...module_path]/` - dynamic module documentation

### Code generation

**library.gen.ts** - generates component library metadata:

```typescript
export const gen = library_gen({on_duplicates: library_throw_on_duplicates});
```

Outputs:
- `library.json` - component metadata, props, dependencies
- `library.ts` - typed export wrapper

**fuz.gen.css.ts** - generates fuz_css utility classes:

```typescript
export const gen = gen_fuz_css();
```

Outputs:
- `fuz.css` - CSS custom properties and utility classes

## Static deployment

Pre-configured for static hosting (GitHub Pages, Netlify, etc.):

- Uses `@sveltejs/adapter-static`
- `prerender = true` in layout
- `static/CNAME` for custom domain
- `static/.nojekyll` for GitHub Pages

Deploy with:

```bash
gro deploy  # builds and pushes to deploy branch
```

## What's included

- Full fuz stack with all imports configured
- Dark/light theme support with persistence
- Theme and color scheme input components
- Documentation structure with API generation
- Static adapter for deployment
- ESLint and Prettier configured
- Vitest for testing
- Example test file

## What's NOT included

- Authentication
- Database
- Server-side functionality (static only)
- Content management

## Project standards

- TypeScript strict mode
- Svelte 5 with runes API
- Prettier with tabs, 100 char width
- Node >= 22.15
- Tests in `src/test/` (not co-located)
- Private package (not published to npm)

## Related projects

- [`fuz_css`](../fuz_css/CLAUDE.md) - CSS framework
- [`fuz_ui`](../fuz_ui/CLAUDE.md) - UI components and docs system
- [`fuz_util`](../fuz_util/CLAUDE.md) - utility functions
- [`fuz_blog`](../fuz_blog/CLAUDE.md) - extends template with blog features
