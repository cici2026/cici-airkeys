<script lang="ts">
	// A coin or token's mark: the network's logo for its own coin, otherwise its symbol on a
	// colour of its own (the same symbol, the same colour). No logos are fetched: the web app
	// asks nothing it does not need.
	import type { Asset } from '#lib/chains/index.ts';
	import Badge from './Badge.svelte';

	let { asset, size = 36 }: { asset: Asset; size?: number } = $props();
	const network = $derived(
		asset.chain.chainId === undefined
			? 'bitcoin'
			: asset.token
				? null
				: asset.symbol === 'ETH'
					? 'evm'
					: null
	);
	const hue = $derived([...asset.symbol].reduce((h, ch) => (h * 31 + ch.charCodeAt(0)) % 360, 17));
	const text = $derived(asset.symbol.replace(/[^A-Za-z0-9]/g, '').slice(0, 4));
</script>

{#if network}
	<Badge {network} {size} />
{:else}
	<span
		class="coin"
		style:width="{size}px"
		style:height="{size}px"
		style:font-size="{Math.round(size * (text.length > 3 ? 0.27 : 0.33))}px"
		style:--hue={hue}
		aria-hidden="true">{text}</span
	>
{/if}

<style>
	.coin {
		flex: none;
		display: grid;
		place-items: center;
		border-radius: 50%;
		font-weight: 700;
		letter-spacing: -0.02em;
		color: hsl(var(--hue) 70% 82%);
		background: hsl(var(--hue) 45% 22%);
		border: 1px solid hsl(var(--hue) 40% 32%);
	}
</style>
