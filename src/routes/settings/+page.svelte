<script lang="ts">
	import NavBar from '$lib/components/NavBar.svelte';
	import Edit05 from '$lib/components/icons/Edit05.svelte';
	import File06 from '$lib/components/icons/File06.svelte';
	import FolderDownload from '$lib/components/icons/FolderDownload.svelte';
	import ShieldTick from '$lib/components/icons/ShieldTick.svelte';
	import MessageQuestionSquare from '$lib/components/icons/MessageQuestionSquare.svelte';
	import Star01 from '$lib/components/icons/Star01.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { Store } from '@tauri-apps/plugin-store';

	let currentDirectory = '';
	let displayName = '';
	let store: Store;

	async function loadSettings() {
		try {
			store = await Store.load('settings.json');

			// Load download directory
			const savedDirectory = await store.get<string>('download_directory');
			if (savedDirectory) {
				await invoke('set_download_directory', { path: savedDirectory });
				currentDirectory = savedDirectory;
			} else {
				currentDirectory = await invoke('get_download_directory');
			}

			// Load display name
			const savedName = await store.get<string>('display_name');
			if (savedName) {
				await invoke('set_display_name', { name: savedName });
				displayName = savedName;
			} else {
				displayName = await invoke('get_display_name');
			}
		} catch (error) {
			console.error('Failed to load settings:', error);
		}
	}

	onMount(loadSettings);

	// Reload settings when page becomes visible (e.g., returning from edit-profile)
	$: if (typeof document !== 'undefined') {
		document.addEventListener('visibilitychange', () => {
			if (!document.hidden) {
				loadSettings();
			}
		});
	}

	async function selectDownloadDirectory() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: 'Select Download Directory'
			});

			if (selected && typeof selected === 'string') {
				await invoke('set_download_directory', { path: selected });
				currentDirectory = selected;

				await store.set('download_directory', selected);
				await store.save();
			}
		} catch (error) {
			console.error('Failed to set download directory:', error);
		}
	}
</script>

<div class="flex w-full flex-col bg-blue-dark-500 p-4">
	<div class="mb-4 flex items-center gap-3">
		<img class="h-8 w-8" src="/logo.png" alt="ARK Drop Logo" />
		<span class="text-lg font-semibold text-white">Settings</span>
	</div>
	<div class="my-2 flex flex-row items-center gap-4 rounded-lg bg-blue-dark-400 p-[10px]">
		<div class="h-10 w-10 overflow-hidden rounded-full border-2 border-white">
			<img src="/images/avatar2.png" alt="" />
		</div>
		<span class="flex-1 text-lg font-semibold text-white">{displayName || 'Loading...'}</span>
		<button
			on:click={() => {
				goto('/settings/edit-profile');
			}}
			class="p rounded-lg border border-button-secondary-border bg-white px-3 py-2"
		>
			<Edit05 class="h-5 w-5 stroke-button-secondary-fg" /></button
		>
	</div>
</div>

<ul
	class="my-3 flex flex-col gap-3 stroke-nav-item-icon-fg p-4 font-semibold text-text-secondary-700"
>
	<li>
		<button
			on:click={selectDownloadDirectory}
			class="flex w-full flex-row items-center gap-3 rounded-lg px-3 py-2 hover:bg-gray-modern-100"
		>
			<FolderDownload class="h-6 w-6 stroke-nav-item-icon-fg" />
			<div class="flex flex-1 flex-col items-start">
				<span>Download Directory</span>
				{#if currentDirectory}
					<span class="text-xs font-normal text-gray-modern-500">{currentDirectory}</span>
				{/if}
			</div>
		</button>
	</li>
	<li class="flex flex-row gap-3 px-3 py-2"><File06 class=" h-6 w-6" />Terms of service</li>
	<li class="flex flex-row gap-3 px-3 py-2">
		<ShieldTick class="h-6 w-6 stroke-nav-item-icon-fg" />Privacy Policy
	</li>
	<li class="flex flex-row gap-3 px-3 py-2">
		<Star01 class="h-6 w-6 stroke-nav-item-icon-fg" />Rate Us
	</li>
	<li class="flex flex-row gap-3 px-3 py-2">
		<MessageQuestionSquare class="h-6 w-6 stroke-nav-item-icon-fg" />Feedback
	</li>
</ul>

<NavBar active="settings" />
