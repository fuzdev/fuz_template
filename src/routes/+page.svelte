<script lang="ts">
	import Svg from '@fuzdev/fuz_ui/Svg.svelte';
	import {fuz_logo} from '@fuzdev/fuz_ui/logos.js';
	import Card from '@fuzdev/fuz_ui/Card.svelte';
	import {resolve} from '$app/paths';
	import {random_item} from '@fuzdev/fuz_util/random.js';
	import ColorSchemeInput from '@fuzdev/fuz_ui/ColorSchemeInput.svelte';
	import ThemeInput from '@fuzdev/fuz_ui/ThemeInput.svelte';

	import Mreows, {items} from '$routes/Mreows.svelte';

	let mreows: Array<{glyph: string}> | undefined = $state([random_item(items), items[4]!]);
</script>

<main>
	<section class="box">
		<header class="box pt_xl">
			<Svg data={fuz_logo} size="var(--icon_size_xl)" />
			<h1 class="mt_xl2">fuz_template</h1>
		</header>
		<Card href={resolve('/about')} icon="">about</Card>
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
