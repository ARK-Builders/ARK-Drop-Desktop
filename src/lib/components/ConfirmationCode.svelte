<script lang="ts">
	import Eye from '$lib/components/icons/Eye.svelte';
	import EyeOff from '$lib/components/icons/EyeOff.svelte';

	export let code = 0;

	let hidden = true;
</script>

<div class="flex flex-col items-center justify-center gap-2">
	<div class="bg-gray-modern-100 flex h-12 w-12 items-center justify-center gap-1 rounded-full">
		{#if hidden}
			<div class="h-2 w-2 rounded-full bg-gray-modern-900"></div>
			<div class="h-2 w-2 rounded-full bg-gray-modern-900"></div>
		{:else}
			<span class="text-lg font-bold text-gray-modern-900">{code}</span>
		{/if}
	</div>
	<button
		class="flex flex-row items-center gap-1"
		on:click={() => {
			hidden = !hidden;
		}}
	>
		{#if hidden}
			<Eye class="h-5 w-5 stroke-blue-dark-500" />
			<span class="text-xs font-medium text-blue-dark-500">Show</span>
		{:else}
			<EyeOff class="h-5 w-5 stroke-blue-dark-500" />
			<span class="text-xs font-medium text-blue-dark-500">Hide</span>
		{/if}
	</button>
</div>
