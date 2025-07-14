<script lang="ts">
	import QrCode from 'qrcode';

	export let hashCode: string | undefined;

	let loading = true;

	let canvas: HTMLCanvasElement;

	$: if (canvas && hashCode) {
		QrCode.toCanvas(canvas, hashCode, {
			scale: 3.6
		})
			.then(() => {
				loading = false;
			})
			.catch((error) => {
				loading = true;
				console.error(error);
			});
	}
</script>

<div class="relative flex h-64 w-64 items-center justify-center">
	<svg
		class="absolute top-0 h-full w-full"
		viewBox="0 0 246 244"
		fill="none"
		xmlns="http://www.w3.org/2000/svg"
	>
		<path
			d="M4 194C4 192.895 3.10457 192 2 192C0.895432 192 0 192.895 0 194H4ZM50 240H10V244H50V240ZM4 234V194H0V234H4ZM10 240C6.68629 240 4 237.314 4 234H0C0 239.523 4.47715 244 10 244V240Z"
			fill="#2970FF"
		/>
		<path
			d="M242 194C242 192.895 242.895 192 244 192C245.105 192 246 192.895 246 194H242ZM196 240H236V244H196V240ZM242 234V194H246V234H242ZM236 240C239.314 240 242 237.314 242 234H246C246 239.523 241.523 244 236 244V240Z"
			fill="#2970FF"
		/>
		<path
			d="M242 54C242 55.1046 242.895 56 244 56C245.105 56 246 55.1046 246 54H242ZM192 4H236V0H192V4ZM242 10V54H246V10H242ZM236 4C239.314 4 242 6.68629 242 10H246C246 4.47715 241.523 0 236 0V4Z"
			fill="#2970FF"
		/>
		<path
			d="M4 50C4 51.1046 3.10457 52 2 52C0.895432 52 0 51.1046 0 50H4ZM50 4H10V0H50V4ZM4 10V50H0V10H4ZM10 4C6.68629 4 4 6.68629 4 10H0C0 4.47715 4.47715 0 10 0V4Z"
			fill="#2970FF"
		/>
	</svg>
	<canvas bind:this={canvas} class="h-full w-full"></canvas>
	{#if loading}
		<div class="flex h-full w-full items-center justify-center">
			<svg
				class="w-1/4 stroke-blue-dark-500 opacity-50"
				stroke="#000"
				viewBox="0 0 24 24"
				xmlns="http://www.w3.org/2000/svg"
				><style>
					.spinner_V8m1 {
						transform-origin: center;
						animation: spinner_zKoa 2s linear infinite;
					}
					.spinner_V8m1 circle {
						stroke-linecap: round;
						animation: spinner_YpZS 1.5s ease-in-out infinite;
					}
					@keyframes spinner_zKoa {
						100% {
							transform: rotate(360deg);
						}
					}
					@keyframes spinner_YpZS {
						0% {
							stroke-dasharray: 0 150;
							stroke-dashoffset: 0;
						}
						47.5% {
							stroke-dasharray: 42 150;
							stroke-dashoffset: -16;
						}
						95%,
						100% {
							stroke-dasharray: 42 150;
							stroke-dashoffset: -59;
						}
					}
				</style><g class="spinner_V8m1"
					><circle cx="12" cy="12" r="9.5" fill="none" stroke-width="2"></circle></g
				></svg
			>
		</div>
	{/if}
</div>
