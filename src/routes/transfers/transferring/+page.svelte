<script>
	import { goto } from '$app/navigation';
	import XClose from '$lib/components/icons/XClose.svelte';
	import FileTransfer from '$lib/components/FileTransfer.svelte';
	import Button from '$lib/components/Button.svelte';
	import PlusCircle from '$lib/components/icons/PlusCircle.svelte';
	import FileType from '$lib/components/icons/FileType.svelte';
	import { formatBytes } from '$lib/util';

	let avatars = ['avatar', 'avatar2'];
</script>

<header class="my-2 flex flex-row justify-between px-4 py-2">
	<button
		on:click={() => {
			goto('/transfers');
		}}
		class="flex flex-row items-center gap-5"
	>
		<XClose class="h-6 w-6 stroke-black" />
		<span class="text-lg font-medium">Transfering Files</span>
	</button>
</header>

<div class="mt-12 flex flex-col items-center justify-center">
	<div class="flex translate-x-[-0.5rem]">
		{#each avatars as avatar}
			<div class="mr-[-1rem] h-16 w-16 overflow-hidden rounded-full border-4 border-white">
				<img src={`/images/${avatar}.png`} alt="" />
			</div>
		{/each}
	</div>
	<span class="mt-3 text-lg font-medium text-gray-modern-900">Wait a moment while transfering</span>
	<span class="mt-1 text-sm text-gray-modern-500"
		>Receiving from <button class="font-semibold text-blue-dark-500 hover:underline">Alice</button
		></span
	>
	<div class="my-6 flex w-11/12 flex-col gap-2">
		<FileTransfer />
		<FileTransfer />
	</div>
	<Button variant="secondary">
		<PlusCircle class="h-5 w-5" />
		Send more</Button
	>
</div>
