<script>
	import { goto } from '$app/navigation';
	import Code from '$lib/components/Code.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';

	let codes = [56, 46, 36];
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

<div class="mt-48 flex w-full flex-col items-center gap-1">
	<span class="text-center text-lg font-medium text-gray-modern-900"
		>Chose the confirmation Code</span
	>
	<span class="text-center text-sm text-gray-modern-500"
		>Make sure code confirmation are matched</span
	>
	<div class="mt-6 flex flex-row gap-6">
		{#each codes as code}
			<Code
				{code}
				on:click={() => {
					if (code === 56) {
						goto('/transfers/transferring');
					} else {
						goto('/transfers/failed');
					}
				}}
			/>
		{/each}
	</div>
</div>
