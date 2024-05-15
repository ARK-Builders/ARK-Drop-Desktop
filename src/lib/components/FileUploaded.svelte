<script lang="ts">
	import type { FileUploaded } from '$lib/types';
	import { formatBytes } from '$lib/util';
	import CheckCircle from './icons/CheckCircle.svelte';
	import FileType from './icons/FileType.svelte';

	export let fileUploaded: FileUploaded;
</script>

<div class="flex w-full flex-col gap-3 rounded-2xl border-1 p-3">
	<div class="flex flex-row items-center gap-3">
		<div class="h-11 w-11 rounded-full border-1 p-[10px]">
			<FileType />
		</div>
		<div class="flex flex-1 flex-col justify-between py-1">
			<span class="text-sm font-medium text-gray-modern-900">{fileUploaded.fileName}</span>
			<p class="flex flex-row items-center gap-1 text-xs text-gray-modern-500">
				{formatBytes(fileUploaded.fileSize)}
			</p>
		</div>
		<CheckCircle class="h-6 w-6 stroke-blue-dark-500" />
	</div>
</div>
