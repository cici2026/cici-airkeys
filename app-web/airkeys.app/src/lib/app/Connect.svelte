<script lang="ts">
	import CameraScan from '$lib/lumen/CameraScan.svelte';
	import { parseMessage } from '$lib/protocol';
	import { NETWORK_NAMES } from '$lib/registry';
	import { addAccounts, conflicts, replaceAccounts, store } from '$lib/state.svelte';

	let { done }: { done: (message: string) => void } = $props();
	let problem = $state('');
	let key = $state(0);

	function received(bytes: Uint8Array) {
		try {
			const m = parseMessage(bytes);
			if (m.type !== 'accounts')
				throw new Error('This is a signed reply, not accounts. On the phone, tap Connect.');
			// an account number this browser knows with another address: another wallet, or the
			// same phrase under another passphrase. The two are never mixed.
			const clash = conflicts(m.accounts);
			if (clash.length) {
				const [a] = clash;
				const replace = confirm(
					`These accounts are of another wallet (or another passphrase): ${NETWORK_NAMES[a.network]} account #${a.index} has another address here.\n\nReplace the ${store.accounts.length} account${store.accounts.length > 1 ? 's' : ''} in this browser with this wallet's? The funds are untouched.`
				);
				if (!replace) {
					done('Nothing added: this browser keeps the accounts it had.');
					return;
				}
				const n = replaceAccounts(m.accounts);
				done(`Replaced the accounts here with this wallet's ${n} account${n > 1 ? 's' : ''}.`);
				return;
			}
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
		<div class="notice danger" role="alert">{problem}</div>
		<div><button onclick={() => ((problem = ''), key++)}>Scan again</button></div>
	{/if}
</div>

<style>
	.narrow {
		max-width: 560px;
	}
</style>
