<script lang="ts">
	import Eye from '$lib/components/icons/Eye.svelte';
	import EyeOff from '$lib/components/icons/EyeOff.svelte';
	import Code from './Code.svelte';

	export let code = 0;

	let hidden = true;
</script>

<div class="flex flex-col items-center justify-center gap-2">
	<Code {hidden} {code} />

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
