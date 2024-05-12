<script lang="ts">
	import { goto } from '$app/navigation';
	import ClockRewind from './icons/ClockRewind.svelte';
	import SwitchVertical02 from './icons/SwitchVertical02.svelte';
	import User03 from './icons/User03.svelte';

	export let active: 'transfers' | 'history' | 'settings' = 'transfers';
</script>

<nav class="absolute bottom-0 left-0 right-0 flex flex-row border-t-[1px] border-gray-modern-200">
	<button
		on:click={() => {
			goto('/transfers');
		}}
		class="flex h-20 flex-1 flex-col items-center justify-center"
	>
		<SwitchVertical02
			class={`h-6 w-6 ${active === 'transfers' ? 'stroke-blue-dark-600' : 'stroke-gray-light-400'} `}
		/>
		<span
			class={`text-xs font-semibold ${active === 'transfers' ? 'text-blue-dark-500' : 'text-gray-modern-500'}`}
			>Transfers</span
		>
	</button>
	<button
		on:click={() => {
			goto('/history');
		}}
		class="flex h-20 flex-1 flex-col items-center justify-center"
	>
		<ClockRewind
			class={`h-6 w-6 ${active === 'history' ? 'stroke-blue-dark-600' : 'stroke-gray-light-400'} `}
		/>
		<span
			class={`text-xs font-semibold ${active === 'history' ? 'text-blue-dark-500' : 'text-gray-modern-500'}`}
			>History</span
		>
	</button>
	<button
		on:click={() => {
			goto('/settings');
		}}
		class="flex h-20 flex-1 flex-col items-center justify-center"
	>
		<User03
			class={`h-6 w-6 ${active === 'settings' ? 'stroke-blue-dark-600' : 'stroke-gray-light-400'} `}
		/>
		<span
			class={`text-xs font-semibold ${active === 'settings' ? 'text-blue-dark-500' : 'text-gray-modern-500'}`}
			>Settings</span
		>
	</button>
</nav>
