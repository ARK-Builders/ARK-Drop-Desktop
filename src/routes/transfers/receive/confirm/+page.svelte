<script>
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';

	export let data;

	let confirmationCode = '';

	async function handleSubmit() {
		if (confirmationCode.trim() === '') return;
		const fullTicket = `${data.hash}:${confirmationCode.trim()}`;

		goto('/transfers/transferring?ticket=' + encodeURIComponent(fullTicket));
	}
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
	<span class="text-center text-lg font-medium text-gray-modern-900">Enter confirmation code</span>
	<span class="text-center text-sm text-gray-modern-500"
		>Enter the confirmation code from the sender</span
	>
	<div class="mt-6 flex w-full max-w-sm flex-col gap-4 px-4">
		<input
			bind:value={confirmationCode}
			type="text"
			placeholder="Confirmation code"
			class="w-full rounded-lg border border-gray-200 p-3 text-center text-lg shadow-lg"
			on:keypress={(e) => {
				if (e.key === 'Enter') {
					handleSubmit();
				}
			}}
		/>
		<Button disabled={confirmationCode.trim() === ''} on:click={handleSubmit}>Continue</Button>
	</div>
</div>
