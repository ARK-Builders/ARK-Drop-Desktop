<script lang="ts">
	import { scan, Format } from '@tauri-apps/plugin-barcode-scanner';
	import { goto } from '$app/navigation';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import { onMount } from 'svelte';
	import QrScanner from 'qr-scanner';
	import { invoke } from '@tauri-apps/api/core';
	import ChevronUp from '$lib/components/icons/ChevronUp.svelte';
	import Button from '$lib/components/Button.svelte';
	import XClose from '$lib/components/icons/XClose.svelte';
	import Clipboard from '$lib/components/icons/Clipboard.svelte';
	import { readText } from '@tauri-apps/plugin-clipboard-manager';

	let videoSource: HTMLVideoElement | null = null;
	let loading = false;
	let qrScanner: QrScanner | null = null;

	let devHash = '';

	let openModal = false;

	onMount(() => {
		scan({ windowed: true, formats: [Format.QRCode] });
	});
</script>

<div class="relative h-full w-full overflow-y-hidden">
	<header class="absolute top-0 z-50 my-2 flex flex-row justify-between px-4 py-2">
		<button
			on:click={() => {
				goto('/transfers');
			}}
			class="flex flex-row items-center gap-5"
		>
			<ChevronLeft class="h-6 w-6 stroke-white" />
			<span class="text-lg font-medium text-white">Back</span>
		</button>
	</header>

	<!-- {#if dev || mode === 'DEBUG'}
<div class="absolute bottom-0 z-50 w-full flex flex-col p-4">
	<span class="italic text-gray-400">Only available in DEV mode</span>
	<textarea class="w-full p-2 rounded-lg" rows="3" bind:value={devHash}></textarea>
	<button
		on:click={() => {
			goto('/transfers/receive/confirm?hash=' + devHash);
		}}
		class="w-full bg-blue-500 text-white rounded-md py-2 mt-2"
	>Receive</button>
</div>
{/if} -->

	<video class="absolute inset-0 z-0 h-full bg-black object-contain" bind:this={videoSource}>
		<track kind="captions" />
	</video>

	<div
		class={`absolute bottom-0 w-full rounded-t-lg bg-white p-4 ${openModal ? '' : 'translate-y-full'} transition-transform`}
	>
		<div class="absolute right-0 top-0">
			<button
				on:click={() => {
					openModal = false;
				}}
				class="p-2"
			>
				<XClose class="h-6 w-6 stroke-black" />
			</button>
		</div>
		<div class="flex flex-col gap-2">
			<span class="font-semibold">Enter Ticket:</span>
			<div class="flex flex-row gap-2">
				<input
					class="w-full rounded-lg border border-gray-200 p-2 shadow-lg"
					bind:value={devHash}
				/>
				<button
					on:click={async () => {
						const clipboardText = await readText();
						if (clipboardText === null) {
							return;
						}
						devHash = clipboardText ?? '';
					}}
					class="flex flex-row items-center border-gray-200 border bg-white  rounded-lg shadow-lg px-3"
				>
					<Clipboard class="h-4 w-4 stroke-black" />
				</button>
			</div>
			<Button disabled={devHash===''} on:click={() => {
				if (devHash === '') return;
				goto('/transfers/receive/confirm?hash=' + devHash);
			}}>Continue</Button>
		</div>
	</div>

	<button
		on:click={() => {
			openModal = true;
		}}
		class={`absolute bottom-0 z-50 flex w-full justify-center ${openModal ? 'hidden' : ''}`}
	>
		<div class="relative m-4 overflow-hidden rounded-full p-2">
			<div class="absolute inset-0 bg-white opacity-25"></div>
			<ChevronUp class="h-8 w-8 stroke-white"></ChevronUp>
		</div>
	</button>
</div>
