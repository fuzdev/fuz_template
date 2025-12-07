<script lang="ts">
	import '@fuzdev/fuz_css/style.css';
	import '@fuzdev/fuz_css/theme.css';
	import '$routes/fuz.css';
	import '$routes/style.css';

	import Themed from '@fuzdev/fuz_ui/Themed.svelte';
	import type {Snippet} from 'svelte';

	const {
		children,
	}: {
		children: Snippet;
	} = $props();
</script>

<svelte:head>
	<title>@fuzdev/fuz_template</title>
</svelte:head>

<Themed>
	{@render children()}
</Themed>
