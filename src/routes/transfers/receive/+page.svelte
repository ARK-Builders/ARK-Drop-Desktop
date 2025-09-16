<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import ChevronUp from '$lib/components/icons/ChevronUp.svelte';
	import Clipboard from '$lib/components/icons/Clipboard.svelte';
	import XClose from '$lib/components/icons/XClose.svelte';
	import { readText } from '@tauri-apps/plugin-clipboard-manager';
	import { Html5Qrcode } from 'html5-qrcode';
	import { onMount } from 'svelte';

	let videoSource: HTMLVideoElement | null = null;
	let loading = false;
	let devHash = '';
	let openModal = false;

	const redirect = (hash: string) => {
		goto('/transfers/receive/confirm?hash=' + hash);
	};

	const qrCodeSuccessCallback = (decodedText: string, _decodedResult: any) => {
		redirect(decodedText);
	};
	const config = { fps: 10, qrbox: { width: 200, height: 200 } };

	onMount(() => {
		const html5QrCode = new Html5Qrcode('reader');

		html5QrCode.start({ facingMode: 'environment' }, config, qrCodeSuccessCallback, () => {});

		() => {
			html5QrCode.stop();
		};
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

	<div class="absolute inset-0 z-0 flex h-full items-center bg-black object-contain">
		<div id="reader" class="aspect-square w-full"></div>
	</div>

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
					class="flex flex-row items-center rounded-lg border border-gray-200 bg-white px-3 shadow-lg"
				>
					<Clipboard class="h-4 w-4 stroke-black" />
				</button>
			</div>
			<Button
				disabled={devHash === ''}
				on:click={() => {
					if (devHash === '') return;
					redirect(devHash);
				}}>Continue</Button
			>
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
