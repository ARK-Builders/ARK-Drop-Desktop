<script lang="ts">
	import { goto } from '$app/navigation';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import { Html5Qrcode } from 'html5-qrcode';
	import { onMount } from 'svelte';

	let videoSource: HTMLVideoElement | null = null;
	let loading = false;

	const qrCodeSuccessCallback = (decodedText: string, _decodedResult: any) => {
		goto('/transfers/recieve/confirm?hash=' + decodedText);
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
