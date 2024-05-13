<script lang="ts">
	export let hidden = false;
	export let code: number;
</script>

<button
	on:click
	class="flex h-12 w-12 cursor-pointer items-center justify-center gap-1 rounded-full bg-gray-modern-100"
>
	{#if hidden}
		<div class="h-2 w-2 rounded-full bg-gray-modern-900"></div>
		<div class="h-2 w-2 rounded-full bg-gray-modern-900"></div>
	{:else}
		<span class="text-lg font-bold text-gray-modern-900">{code}</span>
	{/if}
</button>
