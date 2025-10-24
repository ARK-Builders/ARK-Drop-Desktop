<script lang="ts">
	import NavBar from '$lib/components/NavBar.svelte';
	import Edit05 from '$lib/components/icons/Edit05.svelte';
	import File06 from '$lib/components/icons/File06.svelte';
	import ShieldTick from '$lib/components/icons/ShieldTick.svelte';
	import MessageQuestionSquare from '$lib/components/icons/MessageQuestionSquare.svelte';
	import Star01 from '$lib/components/icons/Star01.svelte';
	import { goto } from '$app/navigation';
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
		<span class="flex-1 text-lg font-semibold text-white">Gilbert</span>
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
	<li class="flex flex-row gap-3 px-3 py-2"><File06 class=" h-6 w-6" />Tems of service</li>
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
