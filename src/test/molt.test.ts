// Twin of the tests in `crates/molt` — verifies the TS ejector's anchors
// against the working tree and exercises both sample plans end to end, so a
// template edit that would break `npm run molt` fails `gro test` (and CI) at
// the same commit, exactly like the Rust twin fails `cargo test`.

import {cpSync, copyFileSync, existsSync, mkdirSync, readFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {assert, describe, test} from 'vitest';

import {
	CLI,
	DOCS,
	FEATURES,
	GITHUB_EXTRAS,
	RUST,
	apply,
	apply_gate,
	build_plan,
	check_all,
	empty_groups,
	json_escape,
	load_templates,
	members_of,
	normalize_remote_url,
	render,
	resolve,
	sample_configs,
	validate_description,
	validate_domain,
	validate_name,
	validate_npm_name,
	verify,
} from '../lib/molt.ts';

const repo_root = fileURLToPath(new URL('../..', import.meta.url));

/** Copies the parts of the repo that molt touches into a scratch dir. */
const copy_template = (destination: string): void => {
	for (const dir of ['src', 'static', '.github', '.cargo', 'crates']) {
		cpSync(join(repo_root, dir), join(destination, dir), {recursive: true});
	}
	for (const file of [
		'package.json',
		'README.md',
		'CLAUDE.md',
		'LICENSE',
		'vite.config.ts',
		'Cargo.toml',
		'Cargo.lock',
		'rust-toolchain.toml',
		'clippy.toml',
	]) {
		copyFileSync(join(repo_root, file), join(destination, file));
	}
};

const scratch_dir = (label: string): string => {
	const dir = join(tmpdir(), `fuz_template_molt_test_ts_${label}_${process.pid}`);
	rmSync(dir, {recursive: true, force: true});
	mkdirSync(dir, {recursive: true});
	return dir;
};

const read = (root: string, path: string): string => readFileSync(join(root, path), 'utf8');

describe('check', () => {
	test('anchors match the template', () => {
		const issues = check_all(repo_root);
		assert.deepEqual(issues, [], `template drifted from molt's anchors:\n${issues.join('\n')}`);
	});
});

describe('apply', () => {
	test('keep-rust sample', () => {
		const [config] = sample_configs();
		const dir = scratch_dir('keep_rust');
		copy_template(dir);

		const plan = build_plan(config, load_templates(dir));
		const issues = verify(dir, plan);
		assert.deepEqual(issues, []);
		apply(dir, plan);

		const package_json = read(dir, 'package.json');
		assert.ok(package_json.includes('"name": "@sample/sample_app"'));
		assert.ok(!package_json.includes('glyph'));
		assert.ok(package_json.includes('"homepage": "https://sample.example.com/"'));
		assert.strictEqual(read(dir, 'static/CNAME'), 'sample.example.com\n');

		// the template's MIT license never ships in a molted project
		assert.ok(!existsSync(join(dir, 'LICENSE')));
		assert.ok(!package_json.includes('"license"'));

		// the TS twin ejector never ships either
		assert.ok(!package_json.includes('"molt"'));
		assert.ok(!existsSync(join(dir, 'src/lib/molt.ts')));
		assert.ok(!existsSync(join(dir, 'src/test/molt.test.ts')));

		const layout = read(dir, 'src/routes/+layout.svelte');
		assert.ok(!layout.includes('logo_fuz_template'));
		// the title carries the project name, not the scoped npm name
		assert.ok(layout.includes('<title>sample_app</title>'));

		const page = read(dir, 'src/routes/+page.svelte');
		assert.ok(!page.includes('Mreows'));
		assert.ok(page.includes('<h1>sample_app</h1>'));
		assert.ok(page.includes("resolve('/docs')"));
		assert.ok(!existsSync(join(dir, 'src/lib/Mreows.svelte')));
		assert.ok(!existsSync(join(dir, 'src/lib/Positioned.svelte')));

		// docs kept
		assert.ok(existsSync(join(dir, 'src/routes/docs')));
		assert.ok(existsSync(join(dir, 'src/routes/library.ts')));

		assert.ok(read(dir, 'README.md').startsWith('# @sample/sample_app'));
		const claude = read(dir, 'CLAUDE.md');
		assert.ok(claude.startsWith('# sample_app'));
		assert.ok(claude.includes('## Rust workspace'));
		assert.ok(claude.includes('src/routes/docs'));

		// github-extras kept: personalized, never the template author's links
		const funding = read(dir, '.github/FUNDING.yml');
		assert.ok(!funding.includes('ryanatkn'));
		assert.ok(funding.includes('your-github-username'));
		const issue_config = read(dir, '.github/ISSUE_TEMPLATE/config.yml');
		assert.ok(issue_config.includes('https://github.com/sample/sample_app/discussions/new/choose'));
		assert.ok(!issue_config.includes('fuzdev/fuz_template'));
		assert.ok(!read(dir, '.github/ISSUE_TEMPLATE/preapproved.md').includes('fuzdev/fuz_template'));
		assert.ok(read(dir, '.github/workflows/check.yml').includes('cargo clippy'));

		// docs kept: the svelte-docinfo tooling stays
		assert.ok(package_json.includes('svelte-docinfo'));
		assert.ok(read(dir, 'vite.config.ts').includes('svelte_docinfo()'));

		const workspace = read(dir, 'Cargo.toml');
		assert.ok(workspace.includes('members = ["crates/sample_app"]'));
		assert.ok(!workspace.includes('license'));
		assert.ok(!existsSync(join(dir, 'crates/molt')));
		assert.ok(!existsSync(join(dir, 'crates/app_cli')));
		assert.ok(!existsSync(join(dir, '.cargo')));
		const crate_manifest = read(dir, 'crates/sample_app/Cargo.toml');
		assert.ok(crate_manifest.includes('name = "sample_app"'));
		// the user's description survives verbatim — the app_cli token rename
		// runs before the description insert
		assert.ok(crate_manifest.includes('description = "a sample app that replaces app_cli"'));
		assert.ok(!crate_manifest.includes('license'));
		const main_rs = read(dir, 'crates/sample_app/src/main.rs');
		assert.ok(main_rs.includes('hello {who}, from sample_app'));
		assert.ok(!main_rs.includes('app_cli'));
		assert.ok(!read(dir, 'crates/sample_app/src/error.rs').includes('app_cli'));

		rmSync(dir, {recursive: true});
	});

	test('strip-rust sample', () => {
		const [, config] = sample_configs();
		const dir = scratch_dir('strip_rust');
		copy_template(dir);

		const plan = build_plan(config, load_templates(dir));
		const issues = verify(dir, plan);
		assert.deepEqual(issues, []);
		apply(dir, plan);

		const package_json = read(dir, 'package.json');
		assert.ok(package_json.includes('"name": "plain_app"'));
		assert.ok(!package_json.includes('homepage'));
		assert.ok(!package_json.includes('repository'));
		assert.ok(!package_json.includes('"license"'));
		assert.ok(!package_json.includes('"molt"'));
		assert.ok(!existsSync(join(dir, 'static/CNAME')));
		assert.ok(!existsSync(join(dir, 'LICENSE')));
		assert.ok(!existsSync(join(dir, 'src/lib/molt.ts')));
		assert.ok(!existsSync(join(dir, 'src/test/molt.test.ts')));

		assert.ok(!existsSync(join(dir, 'Cargo.toml')));
		assert.ok(!existsSync(join(dir, 'Cargo.lock')));
		assert.ok(!existsSync(join(dir, 'crates')));
		assert.ok(!existsSync(join(dir, '.cargo')));
		assert.ok(!existsSync(join(dir, 'rust-toolchain.toml')));
		assert.ok(!existsSync(join(dir, 'clippy.toml')));

		const workflow = read(dir, '.github/workflows/check.yml');
		assert.ok(!workflow.includes('cargo'));
		assert.ok(workflow.includes('npx @fuzdev/gro check'));

		// docs stripped, along with the svelte-docinfo tooling
		assert.ok(!existsSync(join(dir, 'src/routes/docs')));
		assert.ok(!existsSync(join(dir, 'src/routes/library.ts')));
		const page = read(dir, 'src/routes/+page.svelte');
		assert.ok(!page.includes("resolve('/docs')"));
		assert.ok(page.includes("resolve('/about')"));
		assert.ok(!package_json.includes('svelte-docinfo'));
		assert.ok(!read(dir, 'vite.config.ts').includes('svelte_docinfo'));
		assert.ok(!read(dir, 'src/app.d.ts').includes('svelte-docinfo'));

		// extras stripped in this sample
		assert.ok(!existsSync(join(dir, '.github/FUNDING.yml')));
		assert.ok(!existsSync(join(dir, '.github/ISSUE_TEMPLATE')));

		const claude = read(dir, 'CLAUDE.md');
		assert.ok(!read(dir, 'README.md').includes('## rust'));
		assert.ok(!claude.includes('## Rust workspace'));
		assert.ok(!claude.includes('src/routes/docs'));

		rmSync(dir, {recursive: true});
	});
});

describe('apply_gate', () => {
	test('only headless clean wetrun applies ungated', () => {
		assert.strictEqual(apply_gate(true, true, false), 'apply');
		// a terminal always confirms, even with --wetrun — the wizard's
		// answers were just typed, so one keystroke catches a typo
		assert.strictEqual(apply_gate(true, true, true), 'confirm');
		assert.strictEqual(apply_gate(false, true, true), 'confirm');
		// a dirty tree never applies without the dirty-specific confirmation
		assert.strictEqual(apply_gate(true, false, true), 'confirm_dirty');
		assert.strictEqual(apply_gate(false, false, true), 'confirm_dirty');
		// ...and never at all without a terminal
		assert.strictEqual(apply_gate(true, false, false), 'refuse_dirty');
		// non-interactive without --wetrun never writes
		assert.strictEqual(apply_gate(false, true, false), 'dry_run');
		assert.strictEqual(apply_gate(false, false, false), 'dry_run');
	});
});

describe('features', () => {
	test('defaults', () => {
		const {kept, explicit} = resolve([], []);
		assert.deepEqual(Array.from(kept).sort(), [CLI, DOCS, RUST]);
		assert.strictEqual(explicit.size, 0);
	});

	test('csv and repeats', () => {
		const {kept} = resolve(['github-extras,docs'], ['cli']);
		assert.ok(kept.has(GITHUB_EXTRAS));
		assert.ok(kept.has(DOCS));
		assert.ok(kept.has(RUST));
		assert.ok(!kept.has(CLI));
	});

	test('strip rust cascades to cli', () => {
		const {kept} = resolve([], ['rust']);
		assert.ok(!kept.has(RUST));
		assert.ok(!kept.has(CLI));
	});

	test('conflicts error', () => {
		assert.throws(() => resolve(['rust'], ['rust']));
		assert.throws(() => resolve(['cli'], ['rust']));
		assert.throws(() => resolve(['nope'], []));
	});

	test('empty group detection', () => {
		assert.deepEqual(empty_groups(resolve([], []).kept), []);
		assert.deepEqual(empty_groups(resolve([], ['rust']).kept), []);
		// stripping the last member feature while keeping rust is the invalid
		// combination `resolve_config` rejects
		assert.deepEqual(empty_groups(resolve([], ['cli']).kept), [RUST]);
	});

	test('members derive from the registry', () => {
		assert.deepEqual(
			members_of(RUST).map((f) => f.id),
			[CLI],
		);
		assert.strictEqual(members_of(DOCS).length, 0);
		assert.strictEqual(FEATURES.length, 4);
	});
});

describe('config', () => {
	test('name validation', () => {
		validate_name('my_app');
		validate_name('app2');
		validate_name('matcher');
		assert.throws(() => validate_name('My_App'));
		assert.throws(() => validate_name('2app'));
		assert.throws(() => validate_name('my-app'));
		assert.throws(() => validate_name(''));
		assert.throws(() => validate_name('fuz_template'));
		assert.throws(() => validate_name('app_cli'));
		assert.throws(() => validate_name('molt'));
		// Rust keywords and `test` can't be crate names
		assert.throws(() => validate_name('match'));
		assert.throws(() => validate_name('loop'));
		assert.throws(() => validate_name('test'));
		assert.throws(() => validate_name('gen'));
	});

	test('description validation', () => {
		validate_description('');
		validate_description('a fine one-liner');
		assert.throws(() => validate_description('line\nbreak'));
		assert.throws(() => validate_description('tab\there'));
	});

	test('npm name validation', () => {
		validate_npm_name('my_app');
		validate_npm_name('@you/my-app');
		validate_npm_name('my.app2');
		assert.throws(() => validate_npm_name(''));
		assert.throws(() => validate_npm_name('My App'));
		// scope and bare parts are each validated on their own
		assert.throws(() => validate_npm_name('@/my-app'));
		assert.throws(() => validate_npm_name('@you/'));
		assert.throws(() => validate_npm_name('@you'));
		assert.throws(() => validate_npm_name('a@b/c'));
		assert.throws(() => validate_npm_name('you/my-app'));
		assert.throws(() => validate_npm_name('.hidden'));
		assert.throws(() => validate_npm_name('_private'));
		assert.throws(() => validate_npm_name('a'.repeat(215)));
	});

	test('domain validation', () => {
		validate_domain('example.com');
		validate_domain('sub.example.co.uk');
		assert.throws(() => validate_domain('https://example.com'));
		assert.throws(() => validate_domain('nodots'));
	});

	test('json escaping', () => {
		assert.strictEqual(json_escape('plain'), 'plain');
		assert.strictEqual(json_escape('a "b" c'), 'a \\"b\\" c');
		assert.strictEqual(json_escape('line\nbreak'), 'line\\nbreak');
	});
});

describe('templates', () => {
	test('render substitutes all occurrences', () => {
		assert.strictEqual(render('hi __NAME__, __NAME__!', [['__NAME__', 'sam']]), 'hi sam, sam!');
	});

	test('render does not rescan inserted values', () => {
		// a value containing another token must pass through literally
		assert.strictEqual(
			render('a __X__ b __Y__', [
				['__X__', '__Y__'],
				['__Y__', 'z'],
			]),
			'a __Y__ b z',
		);
	});
});

describe('git', () => {
	test('remote url normalization', () => {
		assert.strictEqual(
			normalize_remote_url('git@github.com:you/app.git\n'),
			'https://github.com/you/app',
		);
		assert.strictEqual(
			normalize_remote_url('https://github.com/you/app.git'),
			'https://github.com/you/app',
		);
		assert.strictEqual(
			normalize_remote_url('https://github.com/you/app'),
			'https://github.com/you/app',
		);
		assert.strictEqual(
			normalize_remote_url('ssh://git@github.com/you/app.git'),
			'https://github.com/you/app',
		);
		assert.strictEqual(normalize_remote_url('git@github.com:fuzdev/fuz_template.git'), null);
		assert.strictEqual(normalize_remote_url('https://github.com/fuzdev/fuz_template'), null);
		assert.strictEqual(normalize_remote_url('/local/path'), null);
	});
});
