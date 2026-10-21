<script lang="ts">
	import CameraScan from '$lib/lumen/CameraScan.svelte';
	import { parseMessage } from '$lib/protocol';
	import { addAccounts } from '$lib/state.svelte';

	let { done }: { done: (message: string) => void } = $props();
	let problem = $state('');
	let key = $state(0);

	function received(bytes: Uint8Array) {
		try {
			const m = parseMessage(bytes);
			if (m.type !== 'accounts')
				throw new Error('This is a signed reply, not accounts. On the phone, tap Connect.');
			const n = addAccounts(m.accounts);
			done(n ? `Added ${n} account${n > 1 ? 's' : ''}.` : 'These accounts were already here.');
		} catch (e) {
			problem = (e as Error).message;
		}
	}
</script>

<div class="stack narrow">
	<h2>Scan the phone</h2>
	<p class="dim">
		On the phone: <b>Connect</b> → choose networks → <b>Show code</b>. Hold it in front of this
		camera.
	</p>
	{#key key}
		<CameraScan onmessage={received} />
	{/key}
	{#if problem}
		<div class="notice danger">{problem}</div>
		<div><button onclick={() => ((problem = ''), key++)}>Scan again</button></div>
	{/if}
</div>

<style>
	.narrow {
		max-width: 560px;
	}
</style>
