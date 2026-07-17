// molt — transforms this `fuz_template` clone into your own project, then
// deletes itself. Run `npm run molt` (flags after `--`, e.g.
// `npm run molt -- --help`).
//
// This is the TS twin of the Rust implementation in `crates/molt`, at full
// behavior parity, so ejecting never requires a Rust toolchain. The twins
// share identifier-level naming (sections below mirror the crate's modules)
// and are each self-verifying against the working tree: `src/test/molt.test.ts`
// checks this file's anchors, the crate's tests check its own, and both run
// in CI at the same commit. The output templates in `crates/molt/templates/`
// are single-sourced — compiled into the Rust binary, read at runtime here.
// When you edit an anchored spot in the template, update the anchors on both
// sides in the same change.

import {spawnSync} from 'node:child_process';
import {
	existsSync,
	readFileSync,
	renameSync,
	rmSync,
	statSync,
	unlinkSync,
	writeFileSync,
} from 'node:fs';
import {dirname, join} from 'node:path';
import * as readline from 'node:readline/promises';
import {pathToFileURL} from 'node:url';
import {parseArgs} from 'node:util';

/* error — twin of `crates/molt/src/error.rs` */

export type CliErrorKind = 'usage' | 'precondition' | 'drift' | 'io' | 'terminal';

/**
 * Errors surfaced by molt.
 *
 * Exit-code dialect: `0` success; `2` = the caller must change something
 * local (arguments, git state, modified files) before retrying; `1` =
 * everything else.
 */
export class CliError extends Error {
	kind: CliErrorKind;
	remediation: string | null;

	constructor(kind: CliErrorKind, message: string, remediation: string | null = null) {
		super(message);
		this.kind = kind;
		this.remediation = remediation;
	}

	static usage(message: string): CliError {
		return new CliError('usage', message);
	}

	static precondition(message: string, hint: string | null = null): CliError {
		return new CliError('precondition', message, hint);
	}

	static drift(issues: Array<string>): CliError {
		return new CliError(
			'drift',
			`these files no longer match what molt expects:\n${issues.join('\n')}`,
			'restore the listed files (e.g. git checkout) or run from a fresh clone',
		);
	}

	/** Maps the error to its stable exit code. */
	exit_code(): number {
		return this.kind === 'io' || this.kind === 'terminal' ? 1 : 2;
	}

	/** An optional remediation hint printed under the error. */
	hint(): string | null {
		return this.remediation;
	}
}

/* config — twin of `crates/molt/src/config.rs` */

/**
 * Fully-resolved molt choices, assembled from flags and wizard answers.
 * `kept` holds the feature ids (from `FEATURES`) to keep.
 */
export interface MoltConfig {
	name: string;
	npm_name: string;
	description: string;
	domain: string | null;
	repo_url: string | null;
	kept: Set<string>;
}

const keeps = (config: MoltConfig, feature_id: string): boolean => config.kept.has(feature_id);

// Names cargo refuses as package names: Rust keywords (strict + reserved,
// including 2024's `gen`) plus `test`, which conflicts with the built-in
// test library. The name becomes the starter crate's name, so a keyword
// would leave the molted workspace unable to build.
const RESERVED_CRATE_NAMES = [
	'abstract',
	'as',
	'async',
	'await',
	'become',
	'box',
	'break',
	'const',
	'continue',
	'crate',
	'do',
	'dyn',
	'else',
	'enum',
	'extern',
	'false',
	'final',
	'fn',
	'for',
	'gen',
	'if',
	'impl',
	'in',
	'let',
	'loop',
	'macro',
	'match',
	'mod',
	'move',
	'mut',
	'override',
	'priv',
	'pub',
	'ref',
	'return',
	'self',
	'static',
	'struct',
	'super',
	'test',
	'trait',
	'true',
	'try',
	'type',
	'typeof',
	'unsafe',
	'unsized',
	'use',
	'virtual',
	'where',
	'while',
	'yield',
];

/**
 * Validates a project name: `snake_case`, usable as a crate name.
 *
 * @throws `CliError` when invalid.
 */
export const validate_name = (name: string): void => {
	const valid = /^[a-z][a-z0-9_]*$/.test(name);
	if (!valid) {
		throw CliError.usage(
			`invalid name ${JSON.stringify(name)}: use snake_case starting with a letter (e.g. my_app)`,
		);
	}
	if (RESERVED_CRATE_NAMES.includes(name)) {
		throw CliError.usage(
			`name ${JSON.stringify(name)} can't be used as a crate name (Rust keyword or built-in) — pick another`,
		);
	}
	if (name === 'fuz_template' || name === 'app_cli' || name === 'molt' || name === 'xtask') {
		throw CliError.usage(`name ${JSON.stringify(name)} is reserved — pick your own project name`);
	}
};

/**
 * Validates a project description: a single line, no control characters
 * (it lands in `package.json`, TOML, and markdown blockquotes).
 *
 * @throws `CliError` when invalid.
 */
export const validate_description = (description: string): void => {
	for (const c of description) {
		const code = c.codePointAt(0) ?? 0;
		// the Cc control ranges, mirroring Rust's `char::is_control`
		if (code < 0x20 || (code >= 0x7f && code <= 0x9f)) {
			throw CliError.usage('description must be a single line without control characters');
		}
	}
};

/**
 * Validates an npm package name (scoped names like `@you/name` allowed):
 * lowercase url-safe characters, no `.`/`_` prefix, npm's 214-char limit.
 *
 * @throws `CliError` when invalid.
 */
export const validate_npm_name = (name: string): void => {
	const invalid = (): CliError =>
		CliError.usage(`invalid npm package name ${JSON.stringify(name)}`);
	if (name.length > 214) throw invalid();
	let bare = name;
	if (name.startsWith('@')) {
		const slash = name.indexOf('/');
		if (slash === -1) throw invalid();
		const scope = name.slice(1, slash);
		bare = name.slice(slash + 1);
		if (!npm_name_part_is_valid(scope)) throw invalid();
	}
	if (!npm_name_part_is_valid(bare)) throw invalid();
};

/** Whether a scope or bare package name is valid on its own. */
const npm_name_part_is_valid = (part: string): boolean =>
	part.length > 0 && !part.startsWith('.') && !part.startsWith('_') && /^[a-z0-9._-]+$/.test(part);

/**
 * Validates a bare domain like `example.com` (no scheme, no path).
 *
 * @throws `CliError` when invalid.
 */
export const validate_domain = (domain: string): void => {
	const valid = domain.length > 0 && domain.includes('.') && /^[a-z0-9.-]+$/.test(domain);
	if (!valid) {
		throw CliError.usage(
			`invalid domain ${JSON.stringify(domain)}: expected a bare domain like example.com`,
		);
	}
};

/**
 * Escapes a string for embedding in a JSON string literal (also valid for
 * TOML basic strings, which share the `\"`/`\\`/`\n` escapes).
 */
export const json_escape = (value: string): string => {
	let out = '';
	for (const c of value) {
		const code = c.codePointAt(0) ?? 0;
		if (c === '"') out += '\\"';
		else if (c === '\\') out += '\\\\';
		else if (c === '\n') out += '\\n';
		else if (c === '\t') out += '\\t';
		else if (code < 0x20) out += `\\u${code.toString(16).padStart(4, '0')}`;
		else out += c;
	}
	return out;
};

/* features — twin of `crates/molt/src/features.rs` */

/** A molt-selectable feature of the template. */
export interface Feature {
	/** Stable id used by `--keep`/`--strip` (kebab-case). */
	id: string;
	/** Wizard prompt, phrased as "keep X?". */
	prompt: string;
	/** Whether the feature is kept when unspecified. */
	default_keep: boolean;
	/** A feature this one is part of — stripping the parent strips this too. */
	requires: string | null;
	/**
	 * A feature this one provides a required member for — the parent can't
	 * be kept unless at least one of its members is kept (e.g. cargo
	 * refuses to load a workspace with no member crates).
	 */
	member_of: string | null;
}

export const RUST = 'rust';
export const CLI = 'cli';
export const DOCS = 'docs';
export const GITHUB_EXTRAS = 'github-extras';

export const FEATURES: Array<Feature> = [
	{
		id: RUST,
		prompt: 'keep the Rust workspace? (includes the starter CLI crate, renamed to crates/<name>)',
		default_keep: true,
		requires: null,
		member_of: null,
	},
	{
		// the wizard skips this prompt while `cli` is `rust`'s only member —
		// a kept workspace forces it, so the rust prompt covers the pair
		id: CLI,
		prompt: 'keep the starter CLI crate? (renamed to crates/<name>)',
		default_keep: true,
		requires: RUST,
		member_of: RUST,
	},
	{
		id: DOCS,
		prompt: 'keep the docs system? (src/routes/docs, auto-generated API docs)',
		default_keep: true,
		requires: null,
		member_of: null,
	},
	{
		id: GITHUB_EXTRAS,
		prompt: 'keep .github/FUNDING.yml and the issue templates?',
		default_keep: false,
		requires: null,
		member_of: null,
	},
];

/**
 * Splits repeatable/CSV flag values into feature ids, validating each.
 *
 * @throws `CliError` on an unknown id.
 */
export const parse_ids = (values: Array<string>): Array<string> => {
	const ids: Array<string> = [];
	for (const value of values) {
		for (const raw of value.split(',')) {
			const trimmed = raw.trim();
			if (trimmed === '') continue;
			const feature = FEATURES.find((f) => f.id === trimmed);
			if (!feature) {
				throw CliError.usage(
					`unknown feature ${JSON.stringify(trimmed)} — valid: ${FEATURES.map((f) => f.id).join(', ')}`,
				);
			}
			ids.push(feature.id);
		}
	}
	return ids;
};

/**
 * Resolves the kept-feature set from `--keep`/`--strip` flags, applying
 * defaults and the `requires` cascade. `explicit` returns which features the
 * flags decided (the wizard prompts only for the rest).
 *
 * @throws `CliError` on conflicting flags.
 */
export const resolve = (
	keep: Array<string>,
	strip: Array<string>,
): {kept: Set<string>; explicit: Set<string>} => {
	const keep_ids = parse_ids(keep);
	const strip_ids = parse_ids(strip);
	const both = keep_ids.find((id) => strip_ids.includes(id));
	if (both !== undefined) {
		throw CliError.usage(`feature ${JSON.stringify(both)} passed to both --keep and --strip`);
	}
	for (const id of keep_ids) {
		const feature = FEATURES.find((f) => f.id === id);
		if (feature?.requires && strip_ids.includes(feature.requires)) {
			throw CliError.usage(
				`--keep ${id} conflicts with --strip ${feature.requires} (${id} is part of ${feature.requires})`,
			);
		}
	}
	const kept = new Set<string>();
	const explicit = new Set<string>();
	for (const feature of FEATURES) {
		let choice;
		if (keep_ids.includes(feature.id)) {
			explicit.add(feature.id);
			choice = true;
		} else if (strip_ids.includes(feature.id)) {
			explicit.add(feature.id);
			choice = false;
		} else {
			choice = feature.default_keep;
		}
		if (choice) kept.add(feature.id);
	}
	cascade(kept);
	return {kept, explicit};
};

/**
 * Strips any feature whose `requires` parent is stripped.
 *
 * @mutates kept
 */
export const cascade = (kept: Set<string>): void => {
	for (const feature of FEATURES) {
		if (feature.requires && !kept.has(feature.requires)) {
			kept.delete(feature.id);
		}
	}
};

/** The features that provide required members for `parent`. */
export const members_of = (parent: string): Array<Feature> =>
	FEATURES.filter((f) => f.member_of === parent);

/**
 * Kept features whose required members are all stripped — invalid
 * combinations the caller must reject (or repair by stripping the parent
 * too). Registry order, so callers report deterministically.
 */
export const empty_groups = (kept: Set<string>): Array<string> =>
	FEATURES.filter(
		(parent) =>
			kept.has(parent.id) &&
			members_of(parent.id).length > 0 &&
			!members_of(parent.id).some((m) => kept.has(m.id)),
	).map((parent) => parent.id);

/* git — twin of `crates/molt/src/git.rs` */

/**
 * Runs a git command at `root`, returning its stdout on success and `null`
 * on a nonzero exit (e.g. no `origin` remote configured).
 *
 * @throws `CliError` when git can't be spawned at all.
 */
const git_output = (root: string, args: Array<string>): string | null => {
	const result = spawnSync('git', args, {cwd: root, encoding: 'utf8'});
	if (result.error) {
		throw CliError.precondition(
			`failed to run git: ${result.error.message}`,
			'molt needs git on the PATH',
		);
	}
	return result.status === 0 ? result.stdout : null;
};

/**
 * Normalizes a git remote url (https, scp-style `git@host:`, or
 * `ssh://git@host/`) to an https repository url, returning `null` for the
 * template's own remote (a plain `git clone` of `fuz_template` keeps origin
 * pointed at the template — deriving that would be wrong).
 */
export const normalize_remote_url = (url: string): string | null => {
	const trimmed_url = url.trim();
	if (trimmed_url.includes('fuzdev/fuz_template')) return null;
	let https;
	if (trimmed_url.startsWith('ssh://git@')) {
		https = `https://${trimmed_url.slice('ssh://git@'.length)}`;
	} else if (trimmed_url.startsWith('git@')) {
		https = `https://${trimmed_url.slice('git@'.length).replace(':', '/')}`;
	} else {
		https = trimmed_url;
	}
	const trimmed = https.endsWith('.git') ? https.slice(0, -'.git'.length) : https;
	return trimmed.startsWith('https://') ? trimmed : null;
};

/* anchors — twin of `crates/molt/src/anchors.rs` */
//
// Exact-content anchors into the template's files. molt and the template
// travel in the same repo at the same commit, so these can be exact strings.
// `src/test/molt.test.ts` (and `cargo molt check` for the Rust twin) verify
// every anchor still matches — when you edit an anchored spot in the
// template, update both twins' anchors in the same change.

const PACKAGE_JSON_NAME = '  "name": "@fuzdev/fuz_template",\n';
const PACKAGE_JSON_DESCRIPTION =
	'  "description": "a web app template with TypeScript + SvelteKit + optional Rust for the fuz-stack",\n';
const PACKAGE_JSON_GLYPH = '  "glyph": "❄",\n';
const PACKAGE_JSON_LOGO = '  "logo": "logo.svg",\n';
const PACKAGE_JSON_LOGO_ALT = '  "logo_alt": "a friendly pixelated spider facing you",\n';
const PACKAGE_JSON_LICENSE = '  "license": "MIT",\n';
const PACKAGE_JSON_HOMEPAGE = '  "homepage": "https://template.fuz.dev/",\n';
const PACKAGE_JSON_REPOSITORY = '  "repository": "https://github.com/fuzdev/fuz_template",\n';
// The TS twin ejector's npm script — removed on eject along with the script
// itself and its check test.
const PACKAGE_JSON_MOLT_SCRIPT = '    "molt": "node src/lib/molt.ts",\n';

const LAYOUT_LOGO_IMPORT = "\timport {logo_fuz_template} from '@fuzdev/fuz_ui/logos.ts';\n";
const LAYOUT_SITE_STATE =
	'\t// `glyph` and `repo_url` derive from `pkg_json`; `icon` stays explicit (structured `SvgData`).\n\tsite_context.set(new SiteState({icon: logo_fuz_template, pkg_json}));';
const LAYOUT_SITE_STATE_REPLACEMENT =
	'\t// `glyph` and `repo_url` derive from `pkg_json`.\n\tsite_context.set(new SiteState({pkg_json}));';
const LAYOUT_TITLE = '<title>@fuzdev/fuz_template</title>';

const PAGE_MREOWS_IMPORT = "import Mreows, {mreow_items} from '$lib/Mreows.svelte';";
const H1_FUZ_TEMPLATE = '<h1 class="mt_xl2">fuz_template</h1>';

// the docs system's tooling, stripped with the `docs` feature
const PACKAGE_JSON_SVELTE_DOCINFO = '    "svelte-docinfo": "^0.5.3",\n';
const VITE_DOCINFO_IMPORT = "import svelte_docinfo from 'svelte-docinfo/vite.js';\n";
const VITE_DOCINFO_PLUGIN = 'svelte_docinfo(), ';
const APP_D_TS_DOCINFO =
	'// Registers ambient types for the `virtual:svelte-docinfo` module (Vite plugin).\n// eslint-disable-next-line @typescript-eslint/triple-slash-reference\n/// <reference types="svelte-docinfo/virtual-svelte-docinfo.js" />\n';

// the github extras, personalized when kept
const FUNDING_GITHUB = 'github: ryanatkn';
// The template's repo url as it appears in the issue-template discussion
// links — replaced with the molted project's repo url when derivable.
const TEMPLATE_REPO_URL = 'https://github.com/fuzdev/fuz_template';

const README_H1 = '# @fuzdev/fuz_template ❄';
const CLAUDE_H1 = '# fuz_template\n';
const CNAME_CONTENT = 'template.fuz.dev';
const WORKSPACE_MEMBERS = 'members = ["crates/app_cli", "crates/molt"]';
// The workspace manifest's license line — stripped on eject along with the
// `LICENSE` file (a molted project chooses its own license).
const WORKSPACE_LICENSE = 'license = "MIT"\n';

// The starter CLI crate's name — molt renames every occurrence (and the
// crate directory) to the chosen project name.
const APP_CLI_TOKEN = 'app_cli';
// The starter CLI crate's placeholder description line.
const APP_CLI_DESCRIPTION = 'description = "a CLI scaffolded by fuz_template\'s molt"\n';
// The starter CLI crate's license inheritance line, stripped on eject
// (the workspace's license line goes with it).
const APP_CLI_LICENSE = 'license.workspace = true\n';

// The `rust` job appended to `.github/workflows/check.yml` — kept here as an
// exact-match anchor so stripping the `rust` feature can remove it.
const CI_RUST_JOB = `
  rust:
    # molt anchors this job (crates/molt/src/anchors.rs) so stripping
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
`;

/* templates — twin of `crates/molt/src/templates.rs` */

/**
 * The output templates from `crates/molt/templates/`, substituted with
 * `__PLACEHOLDER__` tokens. The Rust twin embeds these at compile time;
 * here they're read from the working tree (always present pre-molt).
 */
export interface Templates {
	PAGE_SVELTE: string;
	README_MD: string;
	CLAUDE_MD: string;
	README_RUST_SECTION: string;
	CLAUDE_RUST_SECTION: string;
	WORKSPACE_CARGO_TOML: string;
	FUNDING_YML: string;
}

export const load_templates = (root: string): Templates => {
	const template = (filename: string): string =>
		readFileSync(join(root, 'crates/molt/templates', filename), 'utf8');
	return {
		PAGE_SVELTE: template('page.svelte.in'),
		README_MD: template('README.md.in'),
		CLAUDE_MD: template('CLAUDE.md.in'),
		README_RUST_SECTION: template('readme_rust_section.md.in'),
		CLAUDE_RUST_SECTION: template('claude_rust_section.md.in'),
		WORKSPACE_CARGO_TOML: template('workspace_cargo.toml.in'),
		FUNDING_YML: template('funding.yml.in'),
	};
};

/** The starter page's docs link, substituted only when docs are kept. */
const PAGE_DOCS_LINK = " · <a href={resolve('/docs')}>docs</a>";
/** The generated CLAUDE.md's docs bullet, substituted only when docs are kept. */
const CLAUDE_DOCS_BULLET =
	'- `src/routes/docs/` — documentation pages with auto-generated API docs from\n  the `svelte-docinfo` Vite plugin\n';

/**
 * Substitutes `__PLACEHOLDER__` tokens in a template.
 *
 * Single-pass over the template: inserted values are never re-scanned, so
 * user-provided text (a description containing `__RUST_SECTION__`, say)
 * can't corrupt the output.
 */
export const render = (template: string, substitutions: Array<[string, string]>): string => {
	let out = '';
	let rest = template;
	while (rest !== '') {
		let earliest: {idx: number; token: string; value: string} | null = null;
		for (const [token, value] of substitutions) {
			const idx = rest.indexOf(token);
			if (idx !== -1 && (earliest === null || idx < earliest.idx)) {
				earliest = {idx, token, value};
			}
		}
		if (earliest === null) {
			out += rest;
			break;
		}
		out += rest.slice(0, earliest.idx) + earliest.value;
		rest = rest.slice(earliest.idx + earliest.token.length);
	}
	return out;
};

/* plan — twin of `crates/molt/src/plan.rs` */

/**
 * A single filesystem transformation in a molt plan. Paths are relative to
 * the repo root.
 */
export type Action =
	| {kind: 'replace_once'; path: string; anchor: string; replacement: string; label: string}
	| {kind: 'replace_all'; path: string; from: string; to: string; label: string}
	| {kind: 'replace_file'; path: string; anchors: Array<string>; content: string; label: string}
	| {kind: 'rename_dir'; from: string; to: string}
	| {kind: 'delete_file'; path: string}
	| {kind: 'delete_dir'; path: string};

export const describe = (action: Action): string => {
	switch (action.kind) {
		case 'replace_once':
		case 'replace_all':
			return `edit    ${action.path} — ${action.label}`;
		case 'replace_file':
			return `rewrite ${action.path} — ${action.label}`;
		case 'rename_dir':
			return `rename  ${action.from}/ → ${action.to}/`;
		case 'delete_file':
			return `delete  ${action.path}`;
		case 'delete_dir':
			return `delete  ${action.path}/`;
	}
};

const replace_once = (
	path: string,
	anchor: string,
	replacement: string,
	label: string,
): Action => ({
	kind: 'replace_once',
	path,
	anchor,
	replacement,
	label,
});

/** Builds the full molt plan from resolved choices. Pure — reads nothing. */
export const build_plan = (config: MoltConfig, templates: Templates): Array<Action> => {
	const plan: Array<Action> = [];
	const {name, npm_name} = config;

	// package.json identity
	plan.push(
		replace_once(
			'package.json',
			PACKAGE_JSON_NAME,
			`  "name": "${json_escape(npm_name)}",\n`,
			`name → ${npm_name}`,
		),
	);
	const description_replacement =
		config.description === '' ? '' : `  "description": "${json_escape(config.description)}",\n`;
	plan.push(
		replace_once('package.json', PACKAGE_JSON_DESCRIPTION, description_replacement, 'description'),
	);
	for (const [anchor, label] of [
		[PACKAGE_JSON_GLYPH, 'remove template glyph'],
		[PACKAGE_JSON_LOGO, 'remove template logo'],
		[PACKAGE_JSON_LOGO_ALT, 'remove template logo_alt'],
		[PACKAGE_JSON_LICENSE, 'remove license (choose your own)'],
	] as Array<[string, string]>) {
		plan.push(replace_once('package.json', anchor, '', label));
	}

	// the template's MIT license is fuz.dev's, not the new project's
	plan.push({kind: 'delete_file', path: 'LICENSE'});

	// the TS twin ejector (`npm run molt`) is template machinery, deleted on
	// eject like molt's own crate — the script, its npm entry, and its check
	// test (which verifies anchors that no longer match once molted)
	plan.push(replace_once('package.json', PACKAGE_JSON_MOLT_SCRIPT, '', 'remove the molt script'));
	plan.push({kind: 'delete_file', path: 'src/lib/molt.ts'});
	plan.push({kind: 'delete_file', path: 'src/test/molt.test.ts'});

	const homepage_replacement =
		config.domain === null ? '' : `  "homepage": "https://${config.domain}/",\n`;
	plan.push(replace_once('package.json', PACKAGE_JSON_HOMEPAGE, homepage_replacement, 'homepage'));
	const repository_replacement =
		config.repo_url === null ? '' : `  "repository": "${json_escape(config.repo_url)}",\n`;
	plan.push(
		replace_once('package.json', PACKAGE_JSON_REPOSITORY, repository_replacement, 'repository'),
	);

	// custom domain
	if (config.domain !== null) {
		plan.push({
			kind: 'replace_file',
			path: 'static/CNAME',
			anchors: [CNAME_CONTENT],
			content: `${config.domain}\n`,
			label: `custom domain → ${config.domain}`,
		});
	} else {
		plan.push({kind: 'delete_file', path: 'static/CNAME'});
	}

	// root layout: title + template logo
	plan.push(
		replace_once(
			'src/routes/+layout.svelte',
			LAYOUT_LOGO_IMPORT,
			'',
			'remove template logo import',
		),
	);
	plan.push(
		replace_once(
			'src/routes/+layout.svelte',
			LAYOUT_SITE_STATE,
			LAYOUT_SITE_STATE_REPLACEMENT,
			'drop template icon',
		),
	);
	// the project name, not the npm name — a scoped `@you/app` reads badly
	// in a browser tab
	plan.push(
		replace_once(
			'src/routes/+layout.svelte',
			LAYOUT_TITLE,
			`<title>${name}</title>`,
			`title → ${name}`,
		),
	);

	// starter page + demo components
	const docs_link = keeps(config, DOCS) ? PAGE_DOCS_LINK : '';
	plan.push({
		kind: 'replace_file',
		path: 'src/routes/+page.svelte',
		anchors: [PAGE_MREOWS_IMPORT, H1_FUZ_TEMPLATE],
		content: render(templates.PAGE_SVELTE, [
			['__NAME__', name],
			['__DOCS_LINK__', docs_link],
		]),
		label: 'minimal starter page',
	});
	plan.push(
		replace_once(
			'src/routes/about/+page.svelte',
			H1_FUZ_TEMPLATE,
			`<h1 class="mt_xl2">${name}</h1>`,
			`heading → ${name}`,
		),
	);
	plan.push({kind: 'delete_file', path: 'src/lib/Mreows.svelte'});
	plan.push({kind: 'delete_file', path: 'src/lib/Positioned.svelte'});

	// docs system, and the svelte-docinfo tooling that exists only for it
	if (!keeps(config, DOCS)) {
		plan.push({kind: 'delete_dir', path: 'src/routes/docs'});
		plan.push({kind: 'delete_file', path: 'src/routes/library.ts'});
		plan.push(
			replace_once(
				'package.json',
				PACKAGE_JSON_SVELTE_DOCINFO,
				'',
				'remove the svelte-docinfo devDependency',
			),
		);
		plan.push(
			replace_once('vite.config.ts', VITE_DOCINFO_IMPORT, '', 'remove the svelte-docinfo import'),
		);
		plan.push(
			replace_once('vite.config.ts', VITE_DOCINFO_PLUGIN, '', 'remove the svelte-docinfo plugin'),
		);
		plan.push(
			replace_once('src/app.d.ts', APP_D_TS_DOCINFO, '', 'remove the svelte-docinfo ambient types'),
		);
	}

	// regenerated docs
	const description_block = config.description === '' ? '' : `> ${config.description}\n\n`;
	const readme_rust = keeps(config, RUST) ? templates.README_RUST_SECTION : '';
	const claude_rust = keeps(config, RUST) ? templates.CLAUDE_RUST_SECTION : '';
	const claude_docs_bullet = keeps(config, DOCS) ? CLAUDE_DOCS_BULLET : '';
	plan.push({
		kind: 'replace_file',
		path: 'README.md',
		anchors: [README_H1],
		content: render(templates.README_MD, [
			['__NPM_NAME__', npm_name],
			['__DESCRIPTION_BLOCK__', description_block],
			['__RUST_SECTION__', readme_rust],
		]),
		label: 'regenerate for the new project',
	});
	plan.push({
		kind: 'replace_file',
		path: 'CLAUDE.md',
		anchors: [CLAUDE_H1],
		content: render(templates.CLAUDE_MD, [
			['__NAME__', name],
			['__DESCRIPTION_BLOCK__', description_block],
			['__DOCS_BULLET__', claude_docs_bullet],
			['__RUST_SECTION__', claude_rust],
		]),
		label: 'regenerate for the new project (AGENTS.md symlinks here)',
	});

	// .github extras: personalized when kept (the template's funding handles
	// and discussion links must never ship in someone else's project)
	if (keeps(config, GITHUB_EXTRAS)) {
		plan.push({
			kind: 'replace_file',
			path: '.github/FUNDING.yml',
			anchors: [FUNDING_GITHUB],
			content: templates.FUNDING_YML,
			label: 'funding placeholders (fill in or delete)',
		});
		if (config.repo_url !== null) {
			for (const path of [
				'.github/ISSUE_TEMPLATE/config.yml',
				'.github/ISSUE_TEMPLATE/preapproved.md',
			]) {
				plan.push({
					kind: 'replace_all',
					path,
					from: TEMPLATE_REPO_URL,
					to: config.repo_url,
					label: `discussions url → ${config.repo_url}`,
				});
			}
		}
	} else {
		plan.push({kind: 'delete_file', path: '.github/FUNDING.yml'});
		plan.push({kind: 'delete_dir', path: '.github/ISSUE_TEMPLATE'});
	}

	// the Rust workspace, and molt's own crate; `cli` is always kept here —
	// `resolve_config` rejects a kept `rust` with an empty member group,
	// since cargo refuses to load an empty workspace
	if (keeps(config, RUST)) {
		const members = `"crates/${name}"`;
		plan.push({
			kind: 'replace_file',
			path: 'Cargo.toml',
			anchors: [WORKSPACE_MEMBERS],
			content: render(templates.WORKSPACE_CARGO_TOML, [
				['__MEMBERS__', members],
				['__LICENSE__', ''],
			]),
			label: "workspace without molt's crate or the template's license",
		});
		plan.push(
			replace_once(
				'crates/app_cli/Cargo.toml',
				APP_CLI_LICENSE,
				'',
				'remove the license inheritance (the workspace line is gone)',
			),
		);
		// rename the token before inserting the user's description, which may
		// itself contain "app_cli" and must survive verbatim
		for (const path of [
			'crates/app_cli/Cargo.toml',
			'crates/app_cli/src/main.rs',
			'crates/app_cli/src/error.rs',
		]) {
			plan.push({
				kind: 'replace_all',
				path,
				from: APP_CLI_TOKEN,
				to: name,
				label: `${APP_CLI_TOKEN} → ${name}`,
			});
		}
		const crate_description_replacement =
			config.description === '' ? '' : `description = "${json_escape(config.description)}"\n`;
		plan.push(
			replace_once(
				'crates/app_cli/Cargo.toml',
				APP_CLI_DESCRIPTION,
				crate_description_replacement,
				'description',
			),
		);
		plan.push({kind: 'rename_dir', from: 'crates/app_cli', to: `crates/${name}`});
		plan.push({kind: 'delete_dir', path: 'crates/molt'});
	} else {
		plan.push(replace_once('.github/workflows/check.yml', CI_RUST_JOB, '', 'remove the rust job'));
		for (const path of ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'clippy.toml']) {
			plan.push({kind: 'delete_file', path});
		}
		plan.push({kind: 'delete_dir', path: 'crates'});
	}
	plan.push({kind: 'delete_dir', path: '.cargo'});

	// deletes run after every edit and rename, so a mid-apply failure on the
	// `--force` dirty path (the one with no clean undo point) strands as
	// little as possible
	const deletes: Array<Action> = plan.filter(
		(a) => a.kind === 'delete_file' || a.kind === 'delete_dir',
	);
	const ordered: Array<Action> = plan.filter(
		(a) => a.kind !== 'delete_file' && a.kind !== 'delete_dir',
	);
	ordered.push(...deletes);
	return ordered;
};

/**
 * Verifies every action's preconditions against the tree at `root`,
 * returning human-readable issues (empty = the plan is applicable).
 */
export const verify = (root: string, plan: Array<Action>): Array<string> => {
	const issues: Array<string> = [];
	for (const action of plan) {
		switch (action.kind) {
			case 'replace_once': {
				const content = read(root, action.path);
				if (content === null) {
					issues.push(`${action.path}: file missing`);
				} else {
					const count = content.split(action.anchor).length - 1;
					if (count !== 1) {
						issues.push(
							`${action.path}: anchor matched ${count} times (expected exactly 1): ${JSON.stringify(action.anchor)}`,
						);
					}
				}
				break;
			}
			case 'replace_all': {
				const content = read(root, action.path);
				if (content === null) {
					issues.push(`${action.path}: file missing`);
				} else if (!content.includes(action.from)) {
					issues.push(
						`${action.path}: expected occurrences of ${JSON.stringify(action.from)}, found none`,
					);
				}
				break;
			}
			case 'replace_file': {
				const content = read(root, action.path);
				if (content === null) {
					issues.push(`${action.path}: file missing`);
				} else {
					for (const anchor of action.anchors) {
						if (!content.includes(anchor)) {
							issues.push(`${action.path}: expected content not found: ${JSON.stringify(anchor)}`);
						}
					}
				}
				break;
			}
			case 'rename_dir': {
				if (!is_dir(join(root, action.from))) {
					issues.push(`${action.from}: expected a directory to rename`);
				}
				if (existsSync(join(root, action.to))) {
					issues.push(`${action.to}: rename target already exists`);
				}
				break;
			}
			case 'delete_file': {
				if (!is_file(join(root, action.path))) {
					issues.push(`${action.path}: expected a file to delete`);
				}
				break;
			}
			case 'delete_dir': {
				if (!is_dir(join(root, action.path))) {
					issues.push(`${action.path}: expected a directory to delete`);
				}
				break;
			}
		}
	}
	return issues;
};

const read = (root: string, path: string): string | null => {
	try {
		return readFileSync(join(root, path), 'utf8');
	} catch (err) {
		if ((err as NodeJS.ErrnoException).code === 'ENOENT') return null;
		throw err;
	}
};

const is_file = (path: string): boolean => {
	try {
		return statSync(path).isFile();
	} catch {
		return false;
	}
};

const is_dir = (path: string): boolean => {
	try {
		return statSync(path).isDirectory();
	} catch {
		return false;
	}
};

/* apply — twin of `crates/molt/src/apply.rs` */

/**
 * Applies a verified plan at `root`. Callers must run `verify` first —
 * apply assumes anchors match and targets exist.
 */
export const apply = (root: string, plan: Array<Action>): void => {
	for (const action of plan) {
		switch (action.kind) {
			case 'replace_once': {
				const full = join(root, action.path);
				const content = readFileSync(full, 'utf8');
				// function replacement so `$` in user input is never a pattern
				const updated = content.replace(action.anchor, () => action.replacement);
				writeFileSync(full, updated);
				break;
			}
			case 'replace_all': {
				const full = join(root, action.path);
				const content = readFileSync(full, 'utf8');
				const updated = content.replaceAll(action.from, () => action.to);
				writeFileSync(full, updated);
				break;
			}
			case 'replace_file': {
				writeFileSync(join(root, action.path), action.content);
				break;
			}
			case 'rename_dir': {
				renameSync(join(root, action.from), join(root, action.to));
				break;
			}
			case 'delete_file': {
				unlinkSync(join(root, action.path));
				break;
			}
			case 'delete_dir': {
				rmSync(join(root, action.path), {recursive: true});
				break;
			}
		}
	}
};

/* check — twin of `crates/molt/src/check.rs` */

/**
 * Verifies the plans for both sample configs, covering every anchor molt can
 * touch (each feature exercised kept in one config and stripped in the other),
 * plus the embedded-template invariants that anchors alone can't see.
 */
export const check_all = (root: string): Array<string> => {
	const templates = load_templates(root);
	const issues: Array<string> = [];
	for (const config of sample_configs()) {
		issues.push(...verify(root, build_plan(config, templates)));
	}
	// the workspace template must stay byte-identical to the live root
	// Cargo.toml apart from the members and license lines — otherwise an edit
	// to the live lints/profile/deps would silently ship a stale workspace to
	// every molted project while the members anchor still matched
	const live = readFileSync(join(root, 'Cargo.toml'), 'utf8');
	const rendered = render(templates.WORKSPACE_CARGO_TOML, [
		['members = [__MEMBERS__]', WORKSPACE_MEMBERS],
		['__LICENSE__', WORKSPACE_LICENSE],
	]);
	if (live !== rendered) {
		issues.push(
			'Cargo.toml: drifted from crates/molt/templates/workspace_cargo.toml.in (only the members and license lines may differ)',
		);
	}
	issues.sort();
	return issues.filter((issue, i) => issue !== issues[i - 1]);
};

/**
 * Two configs that together exercise every plan branch: one keeps every
 * registry feature (derived from `FEATURES`, so a new feature is covered
 * without touching this) and sets every optional value, one strips every
 * feature and clears the optional values.
 */
export const sample_configs = (): [MoltConfig, MoltConfig] => [
	{
		name: 'sample_app',
		npm_name: '@sample/sample_app',
		// contains "app_cli" to prove the crate rename can't corrupt it
		description: 'a sample app that replaces app_cli',
		domain: 'sample.example.com',
		repo_url: 'https://github.com/sample/sample_app',
		kept: new Set(FEATURES.map((f) => f.id)),
	},
	{
		name: 'plain_app',
		npm_name: 'plain_app',
		description: '',
		domain: null,
		repo_url: null,
		kept: new Set(),
	},
];

/** Runs `molt check`: verifies every anchor and template invariant. */
const check_run = (root: string): number => {
	const issues = check_all(root);
	if (issues.length === 0) {
		console.log('molt check passed: all anchors and embedded templates match');
		return 0;
	}
	console.error(
		"molt check failed — the template drifted from molt's anchors or embedded templates:",
	);
	for (const issue of issues) {
		console.error(`  ${issue}`);
	}
	console.error(
		'(update the anchors in src/lib/molt.ts — and the Rust twin in crates/molt — in the same change)',
	);
	// drift is caller-must-fix, same dialect as CliError kind 'drift'
	return 2;
};

/* wizard — twin of `crates/molt/src/wizard.rs` */

/** Whether both stdin and stdout are terminals, enabling the wizard. */
const interactive = (): boolean => process.stdin.isTTY && process.stdout.isTTY;

let rl: readline.Interface | null = null;

const get_rl = (): readline.Interface => {
	rl ??= readline.createInterface({input: process.stdin, output: process.stdout});
	return rl;
};

/** Asks a question, resolving `null` on EOF (Ctrl-D) instead of hanging. */
const question = (query: string): Promise<string | null> =>
	new Promise((question_resolve) => {
		const iface = get_rl();
		let settled = false;
		iface.question(query).then(
			(line) => {
				settled = true;
				question_resolve(line);
			},
			() => {
				settled = true;
				question_resolve(null);
			},
		);
		iface.once('close', () => {
			if (!settled) question_resolve(null);
		});
	});

/** Prompts for a line; returns the resolved value and whether stdin hit EOF. */
const prompt_raw = async (
	label: string,
	default_value: string | null,
): Promise<{value: string; eof: boolean}> => {
	const suffix = default_value ? ` [${default_value}]` : '';
	const line = await question(`${label}${suffix}: `);
	if (line === null) {
		return {value: default_value ?? '', eof: true};
	}
	const trimmed = line.trim();
	return {value: trimmed === '' ? (default_value ?? '') : trimmed, eof: false};
};

/** Prompts for a line of input; empty input (or EOF) selects `default_value`. */
const prompt = async (label: string, default_value: string | null): Promise<string> =>
	(await prompt_raw(label, default_value)).value;

/**
 * Prompts until `validate` accepts the input, echoing the validation error
 * between attempts. On EOF the error surfaces instead of looping forever.
 */
const prompt_validated = async (
	label: string,
	default_value: string | null,
	validate: (value: string) => void,
): Promise<string> => {
	for (;;) {
		const {value, eof} = await prompt_raw(label, default_value);
		try {
			validate(value);
			return value;
		} catch (err) {
			if (!(err instanceof CliError) || eof) throw err;
			console.log(err.message);
		}
	}
};

/** Prompts for a yes/no answer; empty input (or EOF) selects `default_value`. */
const prompt_bool = async (label: string, default_value: boolean): Promise<boolean> => {
	const suffix = default_value ? '[Y/n]' : '[y/N]';
	for (;;) {
		const line = await question(`${label} ${suffix}: `);
		const answer = (line ?? '').trim().toLowerCase();
		if (line === null || answer === '') return default_value;
		if (answer === 'y' || answer === 'yes') return true;
		if (answer === 'n' || answer === 'no') return false;
		console.log('please answer y or n');
	}
};

/* cli — twin of `crates/molt/src/cli.rs` */

interface TopLevel {
	name: string | null;
	npm_name: string | null;
	description: string | null;
	domain: string | null;
	repo: string | null;
	keep: Array<string>;
	strip: Array<string>;
	wetrun: boolean;
	force: boolean;
	subcommand: 'check' | null;
}

const HELP = `molt — transform this fuz_template clone into your own project, then
molt deletes itself. Run with no arguments for the interactive wizard.
Without a terminal, --name is required and nothing is written unless
--wetrun is passed. Twin of \`cargo molt\` (crates/molt) — same flags.

Usage: npm run molt -- [<flags>] [check]

Flags:
  --name <name>         project name (snake_case; used for crate names,
                        headings, and defaults)
  --npm-name <name>     npm package name (defaults to the project name; may
                        be scoped like @you/name)
  --description <text>  one-line project description
  --domain <domain>     custom domain written to static/CNAME (omit to
                        delete CNAME and homepage)
  --repo <url>          repository url (defaults to the git origin remote
                        when it isn't the template's)
  --keep <ids>          features to keep, comma-separated or repeated
                        (rust, cli, docs, github-extras)
  --strip <ids>         features to strip, comma-separated or repeated
  --wetrun              apply the plan when non-interactive (without it,
                        non-interactive runs write nothing; a terminal
                        always confirms before applying)
  --force               proceed even if the git tree is dirty
  --help                print this help and exit

Subcommands:
  check                 verify molt's anchors and templates still match the
                        template (used by CI and tests)
`;

/**
 * Parses argv into the flag struct, or `null` when `--help` was printed.
 *
 * @throws `CliError` on unknown flags or a bad subcommand.
 */
const parse_top_level = (args: Array<string>): TopLevel | null => {
	let parsed;
	try {
		parsed = parseArgs({
			args,
			options: {
				name: {type: 'string'},
				'npm-name': {type: 'string'},
				description: {type: 'string'},
				domain: {type: 'string'},
				repo: {type: 'string'},
				keep: {type: 'string', multiple: true},
				strip: {type: 'string', multiple: true},
				wetrun: {type: 'boolean'},
				force: {type: 'boolean'},
				help: {type: 'boolean'},
			},
			allowPositionals: true,
		});
	} catch (err) {
		throw CliError.usage(err instanceof Error ? err.message : String(err));
	}
	const {values, positionals} = parsed;
	if (values.help) {
		console.log(HELP);
		return null;
	}
	let subcommand: 'check' | null = null;
	if (positionals.length > 0) {
		if (positionals.length > 1 || positionals[0] !== 'check') {
			throw CliError.usage(`unrecognized arguments: ${positionals.join(' ')}`);
		}
		subcommand = 'check';
	}
	return {
		name: values.name ?? null,
		npm_name: values['npm-name'] ?? null,
		description: values.description ?? null,
		domain: values.domain ?? null,
		repo: values.repo ?? null,
		keep: values.keep ?? [],
		strip: values.strip ?? [],
		wetrun: values.wetrun ?? false,
		force: values.force ?? false,
		subcommand,
	};
};

/**
 * Whether any molt-run flag was passed — they're meaningless combined
 * with the `check` subcommand, so the caller rejects that instead of
 * silently ignoring them.
 */
const has_molt_flags = (top: TopLevel): boolean =>
	top.name !== null ||
	top.npm_name !== null ||
	top.description !== null ||
	top.domain !== null ||
	top.repo !== null ||
	top.keep.length > 0 ||
	top.strip.length > 0 ||
	top.wetrun ||
	top.force;

/* main — twin of `crates/molt/src/main.rs` */

/** Walks up from the current directory to the template's repo root. */
const locate_root = (): string => {
	let dir = process.cwd();
	for (;;) {
		if (is_file(join(dir, 'package.json')) && is_dir(join(dir, 'crates/molt'))) {
			return dir;
		}
		const parent = dirname(dir);
		if (parent === dir) {
			throw CliError.precondition(
				'not inside the fuz_template repo (no package.json + crates/molt found)',
				'run `npm run molt` from your clone of fuz_template',
			);
		}
		dir = parent;
	}
};

/**
 * What stands between a printed plan and applying it, given the run mode.
 *
 * A terminal always gets a confirm prompt — the wizard's answers were just
 * typed, and one keystroke catches a typo'd name before it hits disk. The
 * one combination with no gate at all is `--wetrun` on a clean tree without
 * a terminal — there `git reset --hard && git clean -fd` restores the
 * pre-molt state (the tree was clean, so `git clean` removes only files
 * molt created). A dirty tree (reachable only via `--force`) never applies
 * without the dirty-specific in-the-moment confirmation, and without a
 * terminal it never applies at all: "commit first" is always available, so
 * an override flag would just recreate the hole.
 */
export type ApplyGate = 'apply' | 'confirm' | 'confirm_dirty' | 'dry_run' | 'refuse_dirty';

export const apply_gate = (wetrun: boolean, clean: boolean, is_interactive: boolean): ApplyGate => {
	if (is_interactive) return clean ? 'confirm' : 'confirm_dirty';
	if (wetrun) return clean ? 'apply' : 'refuse_dirty';
	return 'dry_run';
};

const molt = async (top: TopLevel, root: string): Promise<number> => {
	const is_interactive = interactive();

	if (!existsSync(join(root, '.git'))) {
		throw CliError.precondition(
			'not a git repository — molt refuses to run without an undo path',
			"git init && git add -A && git commit -m 'init from fuz_template'",
		);
	}
	// a failed `git status` is its own problem, not a dirty tree
	const status = git_output(root, ['status', '--porcelain']);
	if (status === null) {
		throw CliError.precondition(
			'`git status` failed in this repo',
			'make sure `git status --porcelain` succeeds here, then rerun molt',
		);
	}
	const clean = status.trim() === '';
	if (!clean && !top.force) {
		throw CliError.precondition(
			'the git tree is dirty — molt wants a clean tree so it stays undoable',
			'commit or stash your changes, or pass --force to proceed anyway',
		);
	}
	const gate = apply_gate(top.wetrun, clean, is_interactive);
	if (gate === 'refuse_dirty') {
		// refuse before prompting/planning — this is an invocation problem
		throw CliError.precondition(
			'refusing to apply to a dirty git tree without a terminal — there would be no clean undo point',
			'commit or stash first, or run interactively to confirm the dirty apply',
		);
	}

	const config = await resolve_config(top, root, is_interactive);
	const templates = load_templates(root);
	const plan = build_plan(config, templates);
	const issues = verify(root, plan);
	if (issues.length > 0) throw CliError.drift(issues);

	console.log(`\nmolt plan (${plan.length} actions):`);
	for (const action of plan) {
		console.log(`  ${describe(action)}`);
	}

	// no `refuse_dirty` arm: it returned above, before planning, and the
	// narrowed type proves it (the Rust twin needs an `unreachable!()` here)
	let apply_now: boolean;
	switch (gate) {
		case 'apply': {
			apply_now = true;
			break;
		}
		case 'confirm_dirty': {
			console.log();
			apply_now = await prompt_bool(
				'the git tree is DIRTY — apply anyway, with no clean undo point?',
				false,
			);
			break;
		}
		case 'confirm': {
			console.log();
			apply_now = await prompt_bool(
				'apply this plan? the template becomes your project and molt deletes itself',
				false,
			);
			break;
		}
		case 'dry_run': {
			console.log('\ndry run — nothing written. pass --wetrun to apply.');
			apply_now = false;
			break;
		}
	}
	if (!apply_now) {
		if (gate !== 'dry_run') console.log('declined — nothing written');
		return 0;
	}

	if (gate === 'confirm' || gate === 'confirm_dirty') {
		// the tree may have changed while the prompt waited — apply would
		// silently skip an edit whose anchor disappeared, so verify again
		const reverify_issues = verify(root, plan);
		if (reverify_issues.length > 0) throw CliError.drift(reverify_issues);
	}
	apply(root, plan);
	print_next_steps(config, clean);
	return 0;
};

const resolve_config = async (
	top: TopLevel,
	root: string,
	is_interactive: boolean,
): Promise<MoltConfig> => {
	let name;
	if (top.name !== null) {
		validate_name(top.name);
		name = top.name;
	} else if (is_interactive) {
		name = await prompt_validated('project name', null, validate_name);
	} else {
		throw CliError.usage('--name is required when not running interactively');
	}

	let npm_name;
	if (top.npm_name !== null) {
		validate_npm_name(top.npm_name);
		npm_name = top.npm_name;
	} else if (is_interactive) {
		npm_name = await prompt_validated('npm package name', name, validate_npm_name);
	} else {
		npm_name = name;
	}

	let description;
	if (top.description !== null) {
		description = top.description.trim();
		validate_description(description);
	} else if (is_interactive) {
		description = await prompt('one-line description (optional)', '');
	} else {
		description = '';
	}

	let domain: string | null;
	if (top.domain !== null) {
		domain = non_empty(top.domain);
		if (domain !== null) validate_domain(domain);
	} else if (is_interactive) {
		domain = non_empty(
			await prompt_validated(
				'custom domain like example.com (optional; sets CNAME + homepage)',
				'',
				(value) => {
					const trimmed = value.trim();
					if (trimmed !== '') validate_domain(trimmed);
				},
			),
		);
	} else {
		domain = null;
	}

	const origin = git_output(root, ['remote', 'get-url', 'origin']);
	const derived_repo = origin === null ? null : normalize_remote_url(origin);
	let repo_url: string | null;
	if (top.repo !== null) {
		repo_url = non_empty(top.repo);
	} else if (is_interactive) {
		repo_url = non_empty(await prompt('repository url (optional)', derived_repo));
	} else {
		repo_url = derived_repo;
	}

	const {kept, explicit} = resolve(top.keep, top.strip);
	if (is_interactive) {
		// registry order puts parents before dependents, so `requires` and
		// `member_of` parents are already decided when a dependent comes up
		for (const feature of FEATURES) {
			if (explicit.has(feature.id)) continue;
			if (feature.requires && !kept.has(feature.requires)) {
				kept.delete(feature.id);
				continue;
			}
			// a prompt whose answer explicit flags already force is skipped
			// with a note instead of contradicting the flag after the fact
			const child = FEATURES.find(
				(f) => f.requires === feature.id && explicit.has(f.id) && kept.has(f.id),
			);
			if (child) {
				console.log(`note: keeping ${feature.id} — --keep ${child.id} needs it`);
				kept.add(feature.id);
				continue;
			}
			// a parent whose members were all explicitly stripped can't be
			// kept (e.g. cargo rejects a workspace with no member crates)
			const members = members_of(feature.id);
			if (members.length > 0 && members.every((m) => explicit.has(m.id) && !kept.has(m.id))) {
				const member_ids = members.map((m) => m.id).join(', ');
				console.log(
					`note: --strip ${member_ids} leaves ${feature.id} without a required member — stripping it too`,
				);
				kept.delete(feature.id);
				continue;
			}
			// the sole member of a kept parent rides with the parent's
			// prompt — the parent can't be kept without it, so there is no
			// separate decision to prompt for
			if (
				feature.member_of !== null &&
				kept.has(feature.member_of) &&
				members_of(feature.member_of).length === 1
			) {
				kept.add(feature.id);
				continue;
			}
			if (await prompt_bool(feature.prompt, feature.default_keep)) {
				kept.add(feature.id);
			} else {
				kept.delete(feature.id);
			}
		}
		cascade(kept);
		// a kept parent with every member declined can't build — repair the
		// wizard case (all choices came from prompts); explicit flags are
		// rejected below instead. unreachable while every group has a sole
		// member (the wizard skips that prompt); kept for when a second
		// member returns the prompts
		for (const parent of empty_groups(kept)) {
			if (explicit.has(parent) || members_of(parent).some((m) => explicit.has(m.id))) {
				continue;
			}
			console.log(`note: declining every member of ${parent} leaves it empty — stripping it too`);
			kept.delete(parent);
			cascade(kept);
		}
	}
	const [empty_parent] = empty_groups(kept);
	if (empty_parent !== undefined) {
		const members = members_of(empty_parent)
			.map((m) => m.id)
			.join(', ');
		throw CliError.usage(
			`keeping ${empty_parent} requires at least one of its member features (${members}) — keep one, or strip ${empty_parent} too`,
		);
	}

	return {name, npm_name, description, domain, repo_url, kept};
};

const non_empty = (value: string): string | null => {
	const trimmed = value.trim();
	return trimmed === '' ? null : trimmed;
};

const print_next_steps = (config: MoltConfig, clean: boolean): void => {
	console.log(`\nmolt complete — the project is now ${config.name}. next steps:`);
	console.log('  git status   # review what changed');
	console.log('  npm i        # refresh package-lock.json for the new name');
	console.log('  gro check    # typecheck, test, lint, format');
	if (keeps(config, RUST)) {
		console.log('  cargo check  # refresh Cargo.lock for your crate');
	}
	console.log(`  git add -A && git commit -m "chore: molt fuz_template into ${config.name}"`);
	if (clean) {
		console.log('\nto undo the molt: git reset --hard && git clean -fd');
	}
	console.log(
		"\nstatic/logo.svg and static/favicon.png still carry the template's spider — replace them when ready.",
	);
	console.log(
		"molt deleted the template's MIT LICENSE and license fields — choose your own: https://choosealicense.com/",
	);
	if (keeps(config, GITHUB_EXTRAS)) {
		if (config.repo_url !== null) {
			console.log('.github/FUNDING.yml now holds placeholder funding links — fill in or delete.');
		} else {
			console.log(
				'.github/FUNDING.yml now holds placeholder funding links, and the issue-template discussion links still point at the template (no repo url to derive) — update or delete them.',
			);
		}
	}
};

const main = async (): Promise<number> => {
	try {
		const top = parse_top_level(process.argv.slice(2));
		if (top === null) return 0;
		const root = locate_root();
		if (top.subcommand !== null) {
			if (has_molt_flags(top)) {
				throw CliError.usage('`molt check` takes no other flags');
			}
			return check_run(root);
		}
		return await molt(top, root);
	} catch (err) {
		if (err instanceof CliError) {
			console.error(`error: ${err.message}`);
			const hint = err.hint();
			if (hint) console.error(`hint: ${hint}`);
			return err.exit_code();
		}
		console.error(`error: ${err instanceof Error ? err.message : String(err)}`);
		return 1;
	} finally {
		rl?.close();
	}
};

const entry = process.argv[1];
if (entry && import.meta.url === pathToFileURL(entry).href) {
	process.exitCode = await main();
}
