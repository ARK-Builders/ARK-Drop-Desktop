<script lang="ts">
	import FileType from './icons/FileType.svelte';
	import XClose from './icons/XClose.svelte';
	import CheckCircle from './icons/CheckCircle.svelte';
	import { createEventDispatcher, onMount } from 'svelte';
	import Button from './Button.svelte';
	import { formatBytes, formatTime } from '$lib/util';
	import FileUploaded from './FileUploaded.svelte';
	import type { FileTransfer as FileTransferDTO } from '$lib/types';

	export let file: FileTransferDTO;

	let previousFile = file;

	let internetSpeed = 0; // bytes/s
	let timeLeft = 0; // seconds

	onMount(() => {
		const updateTransfer = () => {
			if (previousFile.transferred !== file.transferred) {
				internetSpeed = file.transferred - previousFile.transferred;
				timeLeft = (file.total - file.transferred) / internetSpeed;
				previousFile = file;
			}
			requestAnimationFrame(updateTransfer);
		}
		requestAnimationFrame(updateTransfer);
	});

	function percentComplete(done: number, all: number) {
		return Math.floor((done / all) * 100);
	}

	const dispatch = createEventDispatcher();

	let openModal = false;

</script>

{#if file.transferred < file.total}
<div class="flex w-full flex-col gap-3 rounded-2xl border-1 p-3">
		<div class="flex flex-row items-center gap-3">
			<div class="h-11 w-11 rounded-full border-1 p-[10px]">
				<FileType />
			</div>
			<div class="flex flex-1 flex-col justify-between py-1">
				<span class="text-sm font-medium text-gray-modern-900">{file.name}</span>
				<p class="flex flex-row items-center gap-1 text-xs text-gray-modern-500">
					{formatBytes(file.transferred)} of {formatBytes(file.total)}
					<svg
						class="fill-gray-modern-500"
						width="4"
						height="4"
						viewBox="0 0 4 4"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
					>
						<circle cx="2" cy="2" r="2" />
					</svg>
					{formatTime(timeLeft)} left
				</p>
			</div>
			<!-- cancel button -->
			<!-- <button
				on:click={() => {
					openModal = true;
				}}
				class="h-6 w-6"
			>
				<XClose class="stroke-blue-dark-500" />
			</button> -->
		</div>

		{#if file.transferred < file.total}
			<div class="relative h-[6px] w-full rounded-full bg-gray-modern-300">
				<div
					style={`--percent-complete: ${100 - percentComplete(file.transferred, file.total)}%`}
					class={`absolute left-0 right-[var(--percent-complete)] h-full rounded-full bg-blue-dark-500`}
				></div>
			</div>
		{/if}
	</div>
{:else}
	<FileUploaded
		fileUploaded={{
			...file,
			recipient: 'Aurora',
			sentAt: new Date()
		}}
	/>
{/if}

{#if openModal}
	<div class="fixed left-0 top-0 h-screen w-screen">
		<div class="absolute inset-0 bg-black opacity-30"></div>
		<div
			class="absolute left-[50%] top-[50%] flex w-10/12 translate-x-[-50%] translate-y-[-50%] flex-col gap-4 rounded-2xl bg-white p-4"
		>
			<div class="flex flex-col gap-2">
				<span class="font-semibold text-gray-modern-900">Cancel this file?</span>
				<span class="text-sm text-gray-modern-500"
					>When you remove this file it cannot be undone</span
				>
			</div>
			<div class="flex flex-row gap-2">
				<div class="border- h-11 w-11 rounded-full border-1 p-[10px]">
					<FileType />
				</div>
				<div class="flex flex-1 flex-col justify-between py-1">
					<span class="text-sm font-medium text-gray-modern-900">{file.name}</span>
					<p class="flex flex-row items-center gap-1 text-xs text-gray-modern-500">
						{formatBytes(file.total)}
					</p>
				</div>
			</div>
			<div class="flex flex-row justify-end gap-3">
				<Button
					on:click={() => {
						openModal = false;
					}}
					class="border-button-secondary-border text-button-secondary-fg"
					size="sm"
					variant="secondary">Cancel</Button
				>
				<Button
					on:click={() => {
						dispatch('cancel');
					}}
					size="sm"
					variant="primary">Remove</Button
				>
			</div>
		</div>
	</div>
{/if}
