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

	listen('sender_progress', (event: Event<{ message: string }>) => {
		const message = event.payload.message;
		const parts = message.split(' ');
		const conn_id = parts[0];
		const action = parts.slice(1).join(' ');

		if (action === 'client connected') {
			connection_id = conn_id;
			status = 'transferring';
			connected = true;
			messages = [...messages, `Client connected`];
			transferredBlobs = 0; 
		} else if (action.startsWith('transfer blob completed')) {
			transferredBlobs += 1;
			messages = [...messages, 'Blob transferred'];
		} else if (action.startsWith('transfer completed')) {
			status = 'completed';
			transferredBlobs = totalBlobs; 
			messages = [...messages, 'Transfer completed'];
		} else if (action === 'transfer aborted') {
			status = 'aborted';
			messages = [...messages, 'Transfer aborted'];
		}
	});
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
				<div class="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-gray-900"></div>
				Waiting to connect...
			</span>
			{#if hashCode}
				<HashCode {hashCode} />
			{/if}
		</div>
	{:else if status === 'transferring'}
		<div class="flex flex-col items-center gap-4">
			<span class="font-medium text-gray-modern-900">Connected to client</span>
			<progress value={transferredBlobs} max={totalBlobs} class="w-64 h-4 rounded"></progress>
			<span class="font-medium text-gray-modern-900">
				{transferredBlobs} of {totalBlobs} files transferred
			</span>
		</div>
	{:else if status === 'completed'}
		<div class="flex flex-col items-center gap-4">
			<span class="font-medium text-green-600">Transfer completed successfully!</span>
		</div>
	{:else if status === 'aborted'}
		<div class="flex flex-col items-center gap-4">
			<span class="font-medium text-red-600">Transfer aborted.</span>
		</div>
	{/if}
</div>