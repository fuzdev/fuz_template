import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import adapter from '@sveltejs/adapter-static';
import { execSync } from 'node:child_process';
import { svelte_preprocess_mdz } from '@fuzdev/mdz/svelte_preprocess_mdz.ts';
import { svelte_preprocess_fuz_code } from '@fuzdev/fuz_code/svelte_preprocess_fuz_code.ts';
import { vite_plugin_fuz_css } from '@fuzdev/fuz_css/vite_plugin_fuz_css.ts';
import svelte_docinfo from 'svelte-docinfo/vite.js';
import { vite_plugin_pkg_json } from '@fuzdev/fuz_ui/vite_plugin_pkg_json.ts';

export default defineConfig({
	plugins: [
		sveltekit({
			preprocess: [svelte_preprocess_mdz(), svelte_preprocess_fuz_code(), vitePreprocess()],
			compilerOptions: { runes: true },
			inspector: true,
			adapter: adapter(),
			paths: { relative: false }, // use root-absolute paths for SSR path comparison: https://svelte.dev/docs/kit/configuration#paths
			version: { name: execSync('git rev-parse HEAD').toString().trim() }

			// Example CSP using https://ui.fuz.dev/docs/csp
			//
			// import {create_csp_directives} from '@fuzdev/fuz_ui/csp.ts';
			// import {csp_directives_of_fuzdev} from '@fuzdev/fuz_ui/csp_of_fuzdev.ts';
			// csp: {
			// 	directives: create_csp_directives({
			// 		extend: [csp_directives_of_fuzdev],
			// 	}),
			// },
		}),
		svelte_docinfo(),
		vite_plugin_fuz_css(),
		vite_plugin_pkg_json()
	],
	optimizeDeps: { exclude: ['@fuzdev/blake3-wasm'] },
	server: {
		// Vite watches the whole root, an inotify watch per file, and the Rust `target/`
		// alone can exhaust the user's `max_user_watches` budget
		watch: { ignored: ['**/target/**', '**/.gro/**'] }
	}
});
