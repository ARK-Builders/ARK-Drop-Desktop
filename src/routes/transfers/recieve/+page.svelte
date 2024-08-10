<script lang="ts">
	import { goto } from '$app/navigation';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import { onMount } from 'svelte';
	import QrScanner from 'qr-scanner';

	let videoSource: HTMLVideoElement | null = null;
	let loading = false;
	let qrScanner: QrScanner | null = null;

	onMount(() => {
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
							goto('/transfers/receive/confirm?hash=' + result.data);
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

<video class="absolute inset-0 z-0 h-full bg-black object-contain" bind:this={videoSource}>
	<track kind="captions" />
</video>
