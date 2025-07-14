<script lang="ts">
	import { goto } from '$app/navigation';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import Scan from '$lib/components/icons/Scan.svelte';
	import Lock03 from '$lib/components/icons/Lock03.svelte';
	import ConfirmationCode from '$lib/components/ConfirmationCode.svelte';
	import QrCode from '$lib/components/QrCode.svelte';
	import { onMount } from 'svelte';
	import HashCode from '$lib/components/HashCode.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getConfirmationCode } from '$lib/util.js';
	import { listen, type Event } from '@tauri-apps/api/event';

	export let data;

	let confirmationCode: number | undefined;
	let hashCode: string | undefined;
	let connection_id: string | null = null;
	let status: 'waiting' | 'transferring' | 'completed' | 'aborted' = 'waiting';
	let messages: string[] = [];
	let connected = false;

	let totalBlobs = data.files.length;
	let transferredBlobs = 0;

	onMount(async () => {
		hashCode = (await invoke('generate_ticket', { paths: data.files })) as string;
		confirmationCode = getConfirmationCode(hashCode);
	});

	listen('send_progress', (event: any) => {
		console.log('SEND', event);
	});

	// listen('sender_progress', (event: Event<{ message: string }>) => {
	// 	const message = event.payload.message;
	// 	const parts = message.split(' ');
	// 	const conn_id = parts[0];
	// 	const action = parts.slice(1).join(' ');

	// 	if (action === 'client connected') {
	// 		connection_id = conn_id;
	// 		status = 'transferring';
	// 		connected = true;
	// 		messages = [...messages, `Client connected`];
	// 		transferredBlobs = 0;
	// 	} else if (action.startsWith('transfer blob completed')) {
	// 		transferredBlobs += 1;
	// 		messages = [...messages, 'Blob transferred'];
	// 	} else if (action.startsWith('transfer completed')) {
	// 		status = 'completed';
	// 		transferredBlobs = totalBlobs;
	// 		messages = [...messages, 'Transfer completed'];
	// 	} else if (action === 'transfer aborted') {
	// 		status = 'aborted';
	// 		messages = [...messages, 'Transfer aborted'];
	// 	}
	// });
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
	<button
		on:click={() => {
			goto('/transfers/receive');
		}}
		class="flex flex-row items-center gap-2"
	>
		<Scan class="h-6 w-6 stroke-blue-dark-500" />
		<span class="text-lg font-medium text-blue-dark-500">Scan</span>
	</button>
</header>

<div class="my-24 flex flex-col items-center justify-center">
	{#if status === 'waiting'}
		<div class="flex flex-col items-center gap-4">
			<div class="flex flex-row items-center gap-2">
				<Lock03 class="h-6 w-6 stroke-black" />
				<span class="text-lg font-medium text-gray-modern-900">Confirmation Code</span>
			</div>
			{#if confirmationCode}
				<ConfirmationCode code={confirmationCode} />
			{/if}
			<div>
				{#if hashCode}
					<QrCode {hashCode} />
				{/if}
			</div>
			<span class="flex items-center gap-2 font-medium text-gray-modern-900">
				Waiting to connect
				<span class="ellipsis-animation"></span>
			</span>
			{#if hashCode}
				<HashCode {hashCode} />
			{/if}
		</div>
	{:else if status === 'transferring'}
		<div class="flex w-full max-w-md flex-col items-center gap-6 bg-white p-6">
			<div class="flex w-full items-center gap-3">
				<div
					class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-blue-50"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-5 w-5 text-blue-500"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"
						/>
					</svg>
				</div>
				<div class="flex-grow">
					<h3 class="font-medium text-gray-900">
						Transfer in progress <span class="ellipsis-animation"></span>
					</h3>
					<p class="text-sm text-gray-500">Connected to client</p>
				</div>
			</div>

			<div class="w-full space-y-2">
				<div class="flex justify-between text-sm font-medium text-gray-700">
					<span>Progress</span>
					<span>{Math.round((transferredBlobs / totalBlobs) * 100)}%</span>
				</div>
				<div class="h-2.5 w-full rounded-full bg-gray-100">
					<div
						class="h-2.5 rounded-full bg-blue-500 transition-all duration-300 ease-out"
						style="width: {Math.round((transferredBlobs / totalBlobs) * 100)}%"
					></div>
				</div>
				<p class="text-right text-xs text-gray-500">
					{transferredBlobs} of {totalBlobs} files transferred
				</p>
			</div>
		</div>
	{:else if status === 'completed'}
		<div class="flex w-full max-w-md flex-col items-center gap-4 rounded-xl bg-white p-8">
			<div class="flex h-16 w-16 items-center justify-center rounded-full bg-green-50">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="h-8 w-8 text-green-500"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M5 13l4 4L19 7"
					/>
				</svg>
			</div>
			<h3 class="text-lg font-medium text-gray-900">Transfer complete!</h3>
			<p class="text-center text-sm text-gray-500">
				All files were successfully transferred to the client.
			</p>
			<button
				on:click={() => {
					goto('/transfers');
				}}
				class="mt-2 rounded-lg bg-green-500 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-green-600"
			>
				Done
			</button>
		</div>
	{:else if status === 'aborted'}
		<div class="flex w-full max-w-md flex-col items-center gap-4 rounded-xl bg-white p-8">
			<div class="flex h-16 w-16 items-center justify-center rounded-full bg-red-50">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="h-8 w-8 text-red-500"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M6 18L18 6M6 6l12 12"
					/>
				</svg>
			</div>
			<h3 class="text-lg font-medium text-gray-900">Transfer aborted</h3>
			<p class="text-center text-sm text-gray-500">
				The file transfer was interrupted or canceled.
			</p>
			<div class="mt-2 flex gap-3">
				<button
					on:click={() => {
						goto('/transfers');
					}}
					class="rounded-lg bg-gray-100 px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-200"
				>
					Close
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.loading-ellipsis span {
		opacity: 0;
	}
	.animate-bounce {
		animation: bounce 1.5s infinite;
	}
	.animation-delay-100 {
		animation-delay: 0.2s;
	}
	.animation-delay-200 {
		animation-delay: 0.4s;
	}
	@keyframes bounce {
		0%,
		100% {
			transform: translateY(0);
			opacity: 0.8;
		}
		50% {
			transform: translateY(-3px);
			opacity: 1;
		}
	}

	.ellipsis-animation:after {
		content: '...';
		animation: ellipsis 1.5s infinite;
		display: inline-block;
		width: 1.5em;
		text-align: left;
	}
	@keyframes ellipsis {
		0% {
			content: '.';
		}
		50% {
			content: '..';
		}
		100% {
			content: '...';
		}
	}
</style>
