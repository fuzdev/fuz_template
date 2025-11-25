<script lang="ts">
	import Svg from '@ryanatkn/fuz/Svg.svelte';
	import {fuz_logo} from '@ryanatkn/fuz/logos.js';
	import Card from '@ryanatkn/fuz/Card.svelte';
	import {resolve} from '$app/paths';
	import {random_item} from '@ryanatkn/belt/random.js';
	import ColorSchemeInput from '@ryanatkn/fuz/ColorSchemeInput.svelte';
	import ThemeInput from '@ryanatkn/fuz/ThemeInput.svelte';

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
