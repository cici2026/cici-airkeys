<script lang="ts">
	import CameraScan from '$lib/lumen/CameraScan.svelte';
	import { parseMessage } from '$lib/protocol';
	import { NETWORK_NAMES } from '$lib/registry';
	import { addAccounts, conflicts, replaceAccounts, store } from '$lib/state.svelte';
	import { t } from '$lib/i18n/index.svelte';

	let { done }: { done: (message: string) => void } = $props();
	let problem = $state('');
	let key = $state(0);

	function received(bytes: Uint8Array) {
		try {
			const m = parseMessage(bytes);
			if (m.type !== 'accounts') throw new Error(t.connect.notAccounts);
			// an account number this browser knows with another address: another wallet, or the
			// same phrase under another passphrase. The two are never mixed.
			const clash = conflicts(m.accounts);
			if (clash.length) {
				const [a] = clash;
				const replace = confirm(
					t.connect.replace(NETWORK_NAMES[a.network], a.index, store.accounts.length)
				);
				if (!replace) {
					done(t.connect.kept);
					return;
				}
				const n = replaceAccounts(m.accounts);
				done(t.connect.replaced(n));
				return;
			}
			const n = addAccounts(m.accounts);
			done(n ? t.connect.added(n) : t.connect.already);
		} catch (e) {
			problem = (e as Error).message;
		}
	}
</script>

<div class="stack narrow">
	<h2>{t.connect.title}</h2>
	<p class="dim">
		{t.connect.leadBefore}<b>{t.connect.leadConnect}</b>{t.connect.leadMiddle}<b
			>{t.connect.leadShow}</b
		>{t.connect.leadAfter}
	</p>
	{#key key}
		<CameraScan onmessage={received} />
	{/key}
	{#if problem}
		<div class="notice danger" role="alert">{problem}</div>
		<div><button onclick={() => ((problem = ''), key++)}>{t.connect.again}</button></div>
	{/if}
</div>

<style>
	.narrow {
		max-width: 560px;
	}
</style>
