<script lang="ts">
	import FileUploaded from '$lib/components/FileUploaded.svelte';
	import NavBar from '$lib/components/NavBar.svelte';
	import FilterLines from '$lib/components/icons/FilterLines.svelte';
	import { getDateInterval } from '$lib/util';

	let historyItems = [
		{
			fileName: 'Img 2718.JPG',
			fileSize: 1_500_000_000, // bytes
			recipient: 'Aurora',
			sentAt: new Date('05-15-2024')
		},
		{
			fileName: 'Img 2718.JPG',
			fileSize: 4_700_000, // bytes
			recipient: 'Aurora',
			sentAt: new Date('05-15-2024')
		},
		{
			fileName: 'Report-Nov-12.docx',
			fileSize: 1_900_000, // bytes
			recipient: 'Aurora',
			sentAt: new Date('05-15-2024')
		},
		{
			fileName: 'Img 2718.JPG',
			fileSize: 4_700_000, // bytes
			recipient: 'Aurora',
			sentAt: new Date('05-15-2024')
		},
		{
			fileName: 'Audio-129.WAV',
			fileSize: 9_400_000, // bytes
			recipient: 'Noah',
			sentAt: new Date('05-14-2024')
		}
	];

	let groupedItems = Object.groupBy(historyItems, ({ sentAt }) => {
		return getDateInterval(sentAt);
	});
</script>

<header class="flex flex-row items-center bg-blue-dark-500 p-4">
	<div class="flex items-center gap-3 flex-1">
		<img class="w-8 h-8" src="/logo.png" alt="ARK Drop Logo" />
		<h1 class="text-lg font-semibold text-white">History</h1>
	</div>
	<button
		class="flex h-9 flex-row items-center gap-2 rounded-lg bg-blue-dark-400 px-3 text-sm font-semibold text-white"
		><FilterLines class="h-5 w-5 stroke-white" /> Recent</button
	>
</header>

<div class="flex flex-col gap-2 p-3">
	{#each Object.entries(groupedItems) as [interval, items]}
		<div class="flex flex-col gap-2">
			<span class="font-semibold text-gray-modern-900">{interval}</span>
			{#each Object.entries(Object.groupBy(items, ({ recipient }) => recipient)) as [recipient, files]}
				<div class="flex flex-col gap-2">
					<h3 class="text-sm text-gray-modern-500">
						Sent to <span class="font-semibold text-blue-dark-500">{recipient}</span>
					</h3>
					{#each files ?? [] as file}
						<FileUploaded fileUploaded={file} />
					{/each}
				</div>
			{/each}
		</div>
	{/each}
</div>

<NavBar active="history" />
