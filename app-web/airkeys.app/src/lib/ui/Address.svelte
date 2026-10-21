<script lang="ts">
	// An address in groups of four, first and last groups bright: how people compare them.
	let { address, size = 15 }: { address: string; size?: number } = $props();
	const prefix = $derived(address.startsWith('0x') ? '0x' : '');
	const groups = $derived((address.slice(prefix.length).match(/.{1,4}/g) ?? []) as string[]);
</script>

<span class="address" style="font-size: {size}px">
	{#if prefix}<span class="faint">0x</span>{/if}
	{#each groups as g, i (i)}<span
			class:edge={i === 0 || i === groups.length - 1}
			class:odd={i % 2 === 1}>{g}</span
		>{/each}
</span>

<style>
	.address {
		font-family: var(--mono);
		line-height: 1.7;
	}
	/* a group never breaks: lines wrap between groups */
	span span {
		display: inline-block;
		white-space: nowrap;
		color: var(--dim);
		margin-right: 0.45em;
	}
	span .odd {
		color: color-mix(in srgb, var(--text) 80%, transparent);
	}
	span .edge {
		color: var(--text);
		font-weight: 650;
	}
	.faint {
		color: var(--faint);
	}
</style>
