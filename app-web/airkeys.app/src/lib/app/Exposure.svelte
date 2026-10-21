<script lang="ts">
	import type { Exposure } from '$lib/chains';
	let { exposure, loading = false }: { exposure: Exposure; loading?: boolean } = $props();
</script>

{#if exposure === 'hidden'}
	<span class="pill safe" title="The public key behind this address has never been revealed">
		<svg viewBox="0 0 24 24"
			><path d="M12 3l7 3v5c0 4.5-3 8.3-7 10-4-1.7-7-5.5-7-10V6z" /><path d="M9 12l2 2 4-4" /></svg
		>Key hidden
	</span>
{:else if exposure === 'exposed'}
	<span class="pill warn" title="This address has sent before: its public key is on chain">
		<svg viewBox="0 0 24 24"><path d="M12 4l9 16H3z" /><path d="M12 10v4M12 17h.01" /></svg>Key
		exposed
	</span>
{:else if loading}
	<span class="pill dim">Checking…</span>
{:else}
	<span
		class="pill dim"
		title="An endpoint did not answer, so it is not known whether this key was revealed"
		>Unknown</span
	>
{/if}

<style>
	svg {
		width: 13px;
		height: 13px;
		fill: none;
		stroke: currentColor;
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
