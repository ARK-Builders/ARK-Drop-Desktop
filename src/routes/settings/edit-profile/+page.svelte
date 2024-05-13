<script>
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import ChevronRight from '$lib/components/icons/ChevronRight.svelte';
</script>

<header class="my-2 flex flex-row justify-between px-4 py-2">
	<button
		on:click={() => {
			goto('/settings');
		}}
		class="flex flex-row items-center gap-5"
	>
		<ChevronLeft class="h-6 w-6 stroke-black" />
		<span class="text-lg font-medium">Edit Profile</span>
	</button>
</header>

<div class="mx-12 my-12 flex flex-col items-center justify-center gap-4">
	<div class="flex flex-col items-center">
		<div class="h-20 w-20 overflow-hidden rounded-full p-2">
			<img class="rounded-full" src="/images/avatar2.png" alt="" />
		</div>
		<button class="flex flex-row items-center gap-1 text-sm text-gray-modern-900"
			>Change Avatar <ChevronRight class="h-5 w-5 stroke-gray-modern-900" /></button
		>
	</div>
	<input class="w-full rounded-lg border-2 border-gray-modern-200 p-1 px-2" type="text" />
	<Button
		size="sm"
		on:click={() => {
			goto('/transfers/recieve/confirm');
		}}
		class="w-full">Save</Button
	>
</div>
