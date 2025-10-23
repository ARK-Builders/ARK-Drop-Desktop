<script lang="ts">
	export let variant: 'primary' | 'secondary' = 'primary';
	export let size: 'sm' | 'md' = 'md';
	export let disabled = false;

	let clazz = '';
	export { clazz as class };
</script>

<button
	{...$$restProps}
	on:click
	class={`flex flex-row items-center justify-center border-2
		${size === 'md' && 'gap-2 rounded-lg px-4 py-[10px] font-medium'}
		${size === 'sm' && 'gap-2 rounded-lg px-3 py-1 font-medium'}
		${variant === 'secondary' && 'border-blue-dark-500 stroke-blue-dark-500 text-blue-dark-500'}
		${variant === 'primary' && 'border-blue-dark-500 bg-blue-dark-500 stroke-white text-white'}
		${disabled && 'cursor-not-allowed opacity-50'}
		${clazz}`}
>
	<slot></slot>
</button>
