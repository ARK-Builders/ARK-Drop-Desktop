<script>
	import { goto } from '$app/navigation';
	import XCircle from '$lib/components/icons/XCircle.svelte';

	import XClose from '$lib/components/icons/XClose.svelte';

	let name = 'Alice';
</script>

<header class="my-2 flex flex-row justify-between px-4 py-2">
	<button
		on:click={() => {
			goto('/transfers');
		}}
		class="flex flex-row items-center gap-5"
	>
		<XClose class="h-6 w-6 stroke-black" />
		<span class="text-lg font-medium">Not Connected</span>
	</button>
</header>

<div class="mt-36 flex flex-col items-center justify-start">
	<XCircle class="h-20 w-20 border-error-500 fill-error-500 stroke-white" />
	<div class="mb-8 mt-3 flex flex-col items-center">
		<span class="text-center text-lg font-medium text-gray-modern-900">Can't connect to {name}</span
		>
		<span class="text-center text-sm text-gray-modern-500"
			>please make sure code confirmation are matched</span
		>
	</div>
	<button
		on:click={() => {
			goto('/transfers');
		}}
		class="rounded-lg border-2 border-blue-dark-500 p-2 px-3 font-medium text-blue-dark-600"
		>Back to home</button
	>
</div>
