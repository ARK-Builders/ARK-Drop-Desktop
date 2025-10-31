<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Store } from '@tauri-apps/plugin-store';
	import Button from '$lib/components/Button.svelte';
	import ChevronLeft from '$lib/components/icons/ChevronLeft.svelte';
	import ChevronRight from '$lib/components/icons/ChevronRight.svelte';
	import XClose from '$lib/components/icons/XClose.svelte';
	import { quintOut } from 'svelte/easing';
	import { fade, fly } from 'svelte/transition';

	export let selectedAvatar = 'avatar2';

	let openAvatars = false;
	let displayName = '';
	let store: Store;

	let avatars = [
		'avatar',
		'avatar2',
		'avatar3',
		'avatar4',
		'avatar5',
		'avatar6',
		'avatar7',
		'avatar8',
		'avatar9',
		'avatar10'
	];

	onMount(async () => {
		try {
			store = await Store.load('settings.json');
			const savedName = await store.get<string>('display_name');

			if (savedName) {
				displayName = savedName;
			} else {
				displayName = await invoke('get_display_name');
			}
		} catch (error) {
			console.error('Failed to load display name:', error);
		}
	});

	async function saveProfile() {
		try {
			const trimmed = displayName.trim();
			if (trimmed) {
				await invoke('set_display_name', { name: trimmed });
				await store.set('display_name', trimmed);
				await store.save();
			}
			goto('/settings');
		} catch (error) {
			console.error('Failed to save profile:', error);
		}
	}
</script>

<header class="my-2 flex flex-row justify-between px-4 py-2">
	<button
		on:click={() => {
			goto('/settings');
		}}
		class="flex flex-row items-center gap-5"
	>
		<ChevronLeft class="h-6 w-6 stroke-black" />
		<span class="text-lg font-medium">Edit Profile</span>
	</button>
</header>

<div class="mx-12 my-12 flex flex-col items-center justify-center gap-4">
	<div class="flex flex-col items-center">
		<div class="h-20 w-20 overflow-hidden rounded-full p-2">
			<img class="rounded-full" src="/images/{selectedAvatar}.png" alt="" />
		</div>
		<button
			on:click={() => {
				openAvatars = true;
			}}
			class="flex flex-row items-center gap-1 text-sm text-gray-modern-900"
			>Change Avatar <ChevronRight class="h-5 w-5 stroke-gray-modern-900" /></button
		>
	</div>
	<input
		bind:value={displayName}
		class="w-full rounded-lg border-2 border-gray-modern-200 p-1 px-2"
		type="text"
		placeholder="Enter your name"
		maxlength="50"
	/>
	<Button size="sm" on:click={saveProfile} class="w-full">Save</Button>
</div>

{#if openAvatars}
	<div transition:fade={{ duration: 100 }} class="fixed left-0 top-0 h-screen w-screen">
		<button
			on:click={() => {
				openAvatars = false;
			}}
			class="absolute inset-0 bg-black opacity-30"
		></button>
		<div
			transition:fly={{ duration: 300, x: 0, y: 500, opacity: 0.5, easing: quintOut }}
			class="absolute bottom-0 flex w-full flex-col gap-4 rounded-t-2xl bg-white p-4"
		>
			<div class="flex flex-row items-center">
				<h2 class="flex-1 text-lg font-medium">Select Avatars</h2>
				<button
					on:click={() => {
						openAvatars = false;
					}}
				>
					<XClose class="h-6 w-6 stroke-black" />
				</button>
			</div>
			<div class="flex h-40 flex-row flex-wrap justify-center gap-3 overflow-y-scroll">
				{#each avatars as avatar}
					<button
						on:click={() => {
							selectedAvatar = avatar;
						}}
						class={`h-16 w-16 overflow-hidden rounded-full border-2 ${avatar === selectedAvatar && 'border-blue-dark-500'}`}
					>
						<img class="rounded-full" src={`/images/${avatar}.png`} alt="" />
					</button>
				{/each}
			</div>
		</div>
	</div>
{/if}
