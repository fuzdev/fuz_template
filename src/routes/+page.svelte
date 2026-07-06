<script lang="ts">
	import Svg from '@fuzdev/fuz_ui/Svg.svelte';
	import {logo_fuz} from '@fuzdev/fuz_ui/logos.ts';
	import {resolve} from '$app/paths';
	import {random_item} from '@fuzdev/fuz_util/random.ts';
	import ColorSchemeInput from '@fuzdev/fuz_ui/ColorSchemeInput.svelte';
	import ThemeInput from '@fuzdev/fuz_ui/ThemeInput.svelte';

	import Mreows, {mreow_items} from '$lib/Mreows.svelte';

	let mreows: Array<{glyph: string}> | undefined = $state.raw([
		random_item(mreow_items),
		mreow_items[4]!,
	]);
</script>

<main>
	<section class="box">
		<header class="box pt_xl">
			<Svg data={logo_fuz} size="var(--icon_size_xl)" />
			<h1 class="mt_xl2">fuz_template</h1>
		</header>
		<div class="column gap_lg">
			<a href={resolve('/about')} class="panel px_xl py_md font_size_xl2 text-align:center">about</a
			>
			<a href={resolve('/docs')} class="panel px_xl py_md font_size_xl2 text-align:center">docs</a>
		</div>
	</section>
	<!-- this section self-removes on eject: molt replaces this whole page -->
	<section class="box panel p_lg width_atmost_sm">
		<p>
			This is the deployed demo of
			<a href="https://github.com/fuzdev/fuz_template">fuz_template</a>, a starter for building apps
			with the <a href="https://www.fuz.dev/">fuz stack</a>. Clone it (or use GitHub's "Use this
			template"), then run <code>cargo molt</code> to make it yours — the readme also covers a manual
			path that skips Rust.
		</p>
	</section>
	<section class="box panel">
		<div class="p_xl box">
			<h2 class="mt_0">color scheme</h2>
			<ColorSchemeInput />
			<h2>theme</h2>
			<ThemeInput />
		</div>
	</section>
	<Mreows bind:mreows />
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		align-items: center;
		margin: 0 auto;
	}
</style>
