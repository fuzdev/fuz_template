<script lang="ts">
	import 'virtual:fuz.css';
	import '@fuzdev/fuz_code/theme.css';
	import '$routes/style.css';

	import ThemeRoot from '@fuzdev/fuz_ui/ThemeRoot.svelte';
	import {SiteState, site_context} from '@fuzdev/fuz_ui/site.svelte.js';
	import {logo_fuz_template} from '@fuzdev/fuz_ui/logos.js';
	import type {Snippet} from 'svelte';
	import pkg_json from 'virtual:pkg.json';

	const {
		children,
	}: {
		children: Snippet;
	} = $props();

	// `glyph` and `repo_url` derive from `pkg_json`; `icon` stays explicit (structured `SvgData`).
	site_context.set(new SiteState({icon: logo_fuz_template, pkg_json}));
</script>

<svelte:head>
	<title>@fuzdev/fuz_template</title>
</svelte:head>

<ThemeRoot>
	{@render children()}
</ThemeRoot>
