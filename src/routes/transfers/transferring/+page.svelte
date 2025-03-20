<script lang="ts">
	import { goto } from '$app/navigation';
	import XClose from '$lib/components/icons/XClose.svelte';
	import FileTransfer from '$lib/components/FileTransfer.svelte';
	import Button from '$lib/components/Button.svelte';
	import PlusCircle from '$lib/components/icons/PlusCircle.svelte';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { formatTime } from '$lib/util.js';
	import { listen } from '@tauri-apps/api/event';
	import type { FileTransfer as FileTransferDTO } from '$lib/types';

	export let data;

	let avatars = ['avatar', 'avatar2'];
	let time_complete: number | undefined = undefined;
	let done = false;
	let output = '';
	let transferFiles: FileTransferDTO[] = [];

	onMount(async () => {
		try {
			let start = Date.now();
			output = await invoke('receive_files', {
				ticket: data.ticket
			});
			time_complete = Date.now() - start;
			done = true;
		} catch (error) {
			console.error("ERROR:", error)
		}

	});

	listen('download_progress', (event) => {
		transferFiles = event.payload as FileTransferDTO[];
	});
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
	{#if done === false}
		<div class="flex translate-x-[-0.5rem]">
			{#each avatars as avatar}
				<div class="mr-[-1rem] h-16 w-16 overflow-hidden rounded-full border-4 border-white">
					<img src={`/images/${avatar}.png`} alt="" />
				</div>
			{/each}
		</div>
	{:else}
		<svg
			class="h-20 w-20 fill-success-500 stroke-white"
			viewBox="0 0 73 72"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
		>
			<path
				fill-rule="evenodd"
				clip-rule="evenodd"
				d="M36.5 3C18.2746 3 3.5 17.7746 3.5 36C3.5 54.2254 18.2746 69 36.5 69C54.7254 69 69.5 54.2254 69.5 36C69.5 17.7746 54.7254 3 36.5 3ZM52.1213 29.1213C53.2929 27.9497 53.2929 26.0503 52.1213 24.8787C50.9497 23.7071 49.0503 23.7071 47.8787 24.8787L32 40.7574L25.1213 33.8787C23.9497 32.7071 22.0503 32.7071 20.8787 33.8787C19.7071 35.0503 19.7071 36.9497 20.8787 38.1213L29.8787 47.1213C31.0503 48.2929 32.9497 48.2929 34.1213 47.1213L52.1213 29.1213Z"
			/>
		</svg>
	{/if}
	{#if done}
		<span class="mt-3 text-lg font-medium text-gray-modern-900">Files Received</span>
	{:else}
		<span class="mt-3 text-lg font-medium text-gray-modern-900"
			>Wait a moment while transfering</span
		>
	{/if}
	{#if done}
		<span class="mt-1 text-sm text-gray-modern-500"
			>Complete in {formatTime((time_complete ?? 0) / 1000)}</span
		>
	{:else}
		<span class="mt-1 text-sm text-gray-modern-500"
			>Receiving from <button class="font-semibold text-blue-dark-500 hover:underline">Alice</button
			></span
		>
	{/if}

	<div class="my-6 flex w-11/12 flex-col gap-2">
		{#each transferFiles as file}
			<FileTransfer
				{file}
				on:cancel={() => {
					goto('/transfers');
				}}
			/>
		{/each}
	</div>
	{#if done}
		<Button
			on:click={() => {
				invoke('open_directory', { directory: output });
			}}
			variant="secondary"
		>
			<PlusCircle class="h-5 w-5" />
			Open in File Manager</Button
		>
	{/if}
</div>
