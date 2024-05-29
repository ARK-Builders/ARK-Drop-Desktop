<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';

	let hashCode: string | undefined;
</script>

<header class="my-2 flex flex-row justify-between px-4 py-2">
	<button
		on:click={() => {
			goto('/transfers');
		}}
		class="flex flex-row items-center gap-5"
	>
		<ChevronLeft class="h-6 w-6 stroke-black" />
		<span class="text-lg font-medium">Back</span>
	</button>
</header>

<h2 class="w-full text-center text-gray-modern-200">Temporary Page</h2>

<div class="mx-12 my-48 flex flex-col items-center justify-center gap-2">
	<span class="w-full text-xs text-gray-modern-600">Enter Hash Code</span>
	<input
		bind:value={hashCode}
		class="w-full rounded-lg border-2 border-gray-modern-200 p-2"
		type="text"
	/>
	<Button
		on:click={() => {
			goto('/transfers/recieve/confirm?hash=' + hashCode);
		}}
		class="w-full ">Connect</Button
	>
</div>
