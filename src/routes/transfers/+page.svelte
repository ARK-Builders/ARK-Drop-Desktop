<script lang="ts">
	import { open } from '@tauri-apps/api/dialog';

	import NavBar from '$lib/components/NavBar.svelte';
	import ArrowCircleBrokenDown from '$lib/components/icons/ArrowCircleBrokenDown.svelte';
	import ArrowCircleBrokenUp from '$lib/components/icons/ArrowCircleBrokenUp.svelte';
	import { goto } from '$app/navigation';
	import Button from '$lib/components/Button.svelte';

	const getSelectedFile = async () => {
		const selected = await open({
			multiple: false
		});

		if (selected === null) {
			return null;
		}

		if (Array.isArray(selected)) {
			return selected[0];
		}

		return selected;
	};
</script>

<header
	class="flex flex-row items-center justify-between border-b-1 border-gray-modern-200 px-4 py-5"
>
	<div class="text-gray-modern-900">
		<h3 class="text-sm">Hi Alice,</h3>
		<h2 class="text-lg font-semibold">Welcome Back</h2>
	</div>
	<img class="h-11 w-11 rounded-full" src="/images/avatar.png" alt="Avatar" />
</header>

<div class="py-6">
	<div class="flex flex-col items-center gap-6 px-4 py-5">
		<img class="w-full max-w-96" src="/images/home.png" alt="Home" />
		<div class="flex flex-col items-center gap-1 text-center">
			<h3 class="text-lg font-medium text-gray-modern-900">Seamless to transfer your files</h3>
			<h4 class="text-sm text-gray-modern-500">
				Simple, fast, and limitless start sharing your files now.
			</h4>
		</div>
	</div>
	<div class="flex flex-row justify-center gap-[1.125rem] p-4">
		<Button
			on:click={async () => {
				const file = await getSelectedFile();
				if (file) {
					const params = new URLSearchParams();
					params.append('file', file);
					goto(`/transfers/send?${params.toString()}`);
				}
			}}
			class="w-32"
		>
			<ArrowCircleBrokenUp class="h-5 w-5 stroke-primary-fg" /><span
				class="text-[16px] font-semibold text-primary-fg">Send</span
			></Button
		>
		<Button
			on:click={() => {
				goto(`/transfers/recieve`);
			}}
			class="w-32"
		>
			<ArrowCircleBrokenDown class="h-5 w-5 stroke-primary-fg" /><span
				class="text-[16px] font-semibold text-primary-fg">Recieve</span
			></Button
		>
	</div>
</div>

<NavBar active="transfers" />
