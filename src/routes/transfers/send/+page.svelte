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
	import { constructQr } from '$lib/util.js';

	export let data;

	let confirmationCode: number | undefined;
	let hashCode: string | undefined;
	let qrCodeData: string | undefined;

	onMount(async () => {
		const ticket = ((await invoke('generate_ticket', { paths: data.files })) as string).split(':');
		hashCode = ticket[0];
		confirmationCode = Number(ticket[1]);
		qrCodeData = constructQr(hashCode, confirmationCode);
	});

	let connected = false;
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
	<div class="mb-10 flex flex-col items-center justify-center gap-2">
		<div class="flex flex-row items-center gap-2">
			<Lock03 class="h-6 w-6 stroke-black" />
			<span class="text-lg font-medium text-gray-modern-900">Confirmation Code</span>
		</div>
		<ConfirmationCode code={confirmationCode} />
	</div>
	<div class="mb-6">
		<QrCode hashCode={qrCodeData} />
	</div>
	<span class="mb-4 font-medium text-gray-modern-900"
		>{connected ? 'Connected' : 'Waiting to connect...'}</span
	>
	<HashCode {hashCode} />
</div>
