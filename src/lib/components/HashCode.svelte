<script lang="ts">
	import { fade } from 'svelte/transition';
	import Link03 from './icons/Link03.svelte';
	export let hashCode: string | undefined;

	let showTooltip = false;
</script>

<button
	on:click={() => {
		navigator.clipboard.writeText(hashCode ?? '');
		showTooltip = true;
		setTimeout(() => {
			showTooltip = false;
		}, 1000);
	}}
	class="flex flex-row items-center gap-1 rounded-lg border-1 border-gray-modern-200 px-3 py-2"
>
	<Link03 class="h-4 w-4 stroke-gray-modern-600" />
	<span class="text-xs text-gray-modern-600">Hash Code</span>
</button>

{#if showTooltip}
	<div
		out:fade
		class="fixed bottom-4 flex w-56 items-center justify-center rounded-xl bg-gray-modern-900 p-2 text-xs text-gray-modern-500"
	>
		Hash Code copied to clipboard
	</div>
{/if}
