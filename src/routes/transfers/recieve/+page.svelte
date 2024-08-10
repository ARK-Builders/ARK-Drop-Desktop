<script lang="ts">
	import { goto } from '$app/navigation';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import { onMount } from 'svelte';
	import QrScanner from 'qr-scanner';
	import { invoke } from '@tauri-apps/api';
	import { dev } from '$app/environment';

	let videoSource: HTMLVideoElement | null = null;
	let loading = false;
	let qrScanner: QrScanner | null = null;

	let devHash = '';

	let mode = '';

	onMount(() => {

		invoke<string>("get_env", { key: "MODE" }).then((res) => {
			mode = res;
		});

		navigator.permissions.query({ name: 'camera' as PermissionName }).then((permission) => {
			if (permission.state === 'denied') {
				goto('/transfers');
			}

			loading = true;

			navigator.mediaDevices
				.getUserMedia({
					audio: false,
					video: true
				})
				.then((stream) => {
					if (!videoSource) return;

					videoSource.srcObject = stream;
					videoSource.play();
					loading = false;

					qrScanner = new QrScanner(
						videoSource,
						(result) => {
							goto('/transfers/recieve/confirm?hash=' + result.data);
						},
						{
							highlightCodeOutline: true,
							highlightScanRegion: true
						}
					);

					qrScanner.start();
				});
		});

		return () => {
			if (qrScanner) {
				qrScanner.stop();
			}
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

{#if dev || mode === 'DEBUG'}
<div class="absolute bottom-0 z-50 w-full flex flex-col p-4">
	<span class="italic text-gray-400">Only available in DEV mode</span>
	<textarea class="w-full p-2 rounded-lg" rows="3" bind:value={devHash}></textarea>
	<button
		on:click={() => {
			goto('/transfers/recieve/confirm?hash=' + devHash);
		}}
		class="w-full bg-blue-500 text-white rounded-md py-2 mt-2"
	>Recieve</button>
</div>
{/if}



<video class="absolute inset-0 z-0 h-full bg-black object-contain" bind:this={videoSource}>
	<track kind="captions" />
</video>
