<script>
	import NavBar from '$lib/components/NavBar.svelte';
	import ArrowCircleBrokenDown from '$lib/components/icons/ArrowCircleBrokenDown.svelte';
	import ArrowCircleBrokenUp from '$lib/components/icons/ArrowCircleBrokenUp.svelte';
	import ClockRewind from '$lib/components/icons/ClockRewind.svelte';
	import SwitchVertical02 from '$lib/components/icons/SwitchVertical02.svelte';
	import User03 from '$lib/components/icons/User03.svelte';
</script>

<header
	class="flex flex-row items-center justify-between border-b-[1px] border-gray-modern-200 px-4 py-5"
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
		<button
			class="flex w-32 flex-row items-center justify-center gap-2 rounded-lg bg-blue-dark-500 px-[1.125rem] py-3"
		>
			<ArrowCircleBrokenUp class="h-5 w-5 stroke-primary-fg" /><span
				class="text-[16px] font-semibold text-primary-fg">Send</span
			></button
		>
		<button
			class="flex w-32 flex-row items-center justify-center gap-2 rounded-lg bg-blue-dark-500 px-[1.125rem] py-3"
		>
			<ArrowCircleBrokenDown class="h-5 w-5 stroke-primary-fg" /><span
				class="text-[16px] font-semibold text-primary-fg">Recieve</span
			></button
		>
	</div>
</div>

<NavBar active="transfers" />
