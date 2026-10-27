<script lang="ts">
	// Every account the phone has shown, grouped by account number, with what each holds.
	import { NETWORK_NAMES, type Network } from '#lib/registry.ts';
	import { store } from '#lib/state.svelte.ts';
	import { ensure, holdingOf, refresh } from '#lib/holdings.svelte.ts';
	import { formatShort } from '#lib/units.ts';
	import type { Account } from '#lib/protocol.ts';
	import Badge from '#lib/ui/Badge.svelte';
	import Exposure from './Exposure.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';

	let { open, connect }: { open: (a: Account) => void; connect: () => void } = $props();
	// Bitcoin has no test network here: on Testnet only the EVM accounts show
	const order = $derived<Network[]>(store.settings.testnet ? ['evm'] : ['bitcoin', 'evm']);
	const shownAccounts = $derived(store.accounts.filter((a) => order.includes(a.network)));
	const groups = $derived(
		[...new Set(shownAccounts.map((a) => a.index))]
			.sort((a, b) => a - b)
			.map((index) => ({ index, accounts: shownAccounts.filter((a) => a.index === index) }))
	);
	$effect(() => ensure(...shownAccounts));

	// with many accounts (the phone can show all 100 at once), the empty ones fold into one line
	const loaded = (a: Account) => {
		const h = holdingOf(a);
		return !!h && !h.loading;
	};
	// nothing on it: read and empty, or not read (a Bitcoin account past the gap)
	const empty = (g: { accounts: Account[] }) =>
		g.accounts.every((a) => {
			const h = holdingOf(a);
			return (
				!!h &&
				!h.loading &&
				(h.skipped || (!h.failed.length && h.assets.every((x) => x.amount === 0n)))
			);
		});
	const skipped = $derived(shownAccounts.filter((a) => holdingOf(a)?.skipped));
	// the first account always shows: where to receive, when nothing holds anything yet
	const emptyGroups = $derived(groups.slice(1).filter(empty));
	const loading = $derived(shownAccounts.filter((a) => !loaded(a)).length);
	let showEmpty = $state(false);
	const shown = $derived(showEmpty ? groups : groups.filter((g, k) => k === 0 || !empty(g)));

	/** What the account holds, a short line per chain: "Ethereum · 5.77 ETH, 37.19 USDC +3". At
	 * most three chains; the rest are counted. */
	function summary(a: Account): string[] {
		const h = holdingOf(a);
		if (!h) return ['…'];
		if (h.skipped) return [t.wallet.notChecked];
		const held = h.assets.filter((x) => x.amount > 0n);
		if (!held.length) return [h.loading ? '…' : t.wallet.empty];
		const chains = [...new Set(held.map((x) => x.chain.name))];
		const lines = chains.slice(0, 3).map((c) => {
			const here = held.filter((x) => x.chain.name === c);
			const list = here
				.slice(0, 2)
				.map((x) => `${formatShort(x.amount, x.decimals)} ${x.symbol}`)
				.join(t.common.comma);
			const more = here.length > 2 ? ` +${here.length - 2}` : '';
			return a.network === 'evm' ? `${c} · ${list}${more}` : `${list}${more}`;
		});
		if (chains.length > 3) lines.push(t.wallet.moreChains(chains.length - 3));
		return lines;
	}
</script>

{#if !store.accounts.length}
	<div class="empty card stack">
		<h2>{t.wallet.connectTitle}</h2>
		<p class="dim">{t.wallet.connectLead}</p>
		<ol class="dim">
			<li>
				{t.wallet.step1Before}<b>{t.wallet.step1Connect}</b>{t.wallet.step1Middle}<b
					>{t.wallet.step1Show}</b
				>
			</li>
			<li>{t.wallet.step2Before}<b>{t.wallet.step2Scan}</b>{t.wallet.step2After}</li>
		</ol>
		<div><button class="primary" onclick={connect}>{t.wallet.scanThePhone}</button></div>
	</div>
{:else}
	<div class="stack">
		<div class="row small">
			<span class="grow">
				{t.wallet.counts(groups.length, shownAccounts.length)}{loading
					? t.wallet.readingLeft(loading)
					: ''}
			</span>
			<button
				class="ghost small"
				disabled={loading > 0}
				onclick={() => refresh(shownAccounts, true)}>{t.common.refresh}</button
			>
		</div>
		{#if store.settings.testnet && store.accounts.some((a) => a.network === 'bitcoin')}
			<p class="small">{t.wallet.bitcoinMainnetOnly}</p>
		{/if}
		{#each shown as g (g.index)}
			<section class="stack">
				<div class="label">{t.wallet.account(g.index)}</div>
				{#each order as n (n)}
					{#each g.accounts.filter((a) => a.network === n) as a (a.address)}
						<button class="account card" onclick={() => open(a)}>
							<Badge network={a.network} />
							<div class="grow">
								<div class="row">
									<b>{NETWORK_NAMES[a.network]}</b><span class="mono small"
										>{a.address.slice(0, 8)}…{a.address.slice(-6)}</span
									>
								</div>
								{#each summary(a) as line, k (k)}<div class="small line">{line}</div>{/each}
							</div>
							<Exposure
								exposure={holdingOf(a)?.exposure ?? 'unknown'}
								loading={!holdingOf(a) || holdingOf(a)!.loading}
							/>
						</button>
					{/each}
				{/each}
			</section>
		{/each}
		{#if emptyGroups.length && !showEmpty}
			<button class="ghost folded" onclick={() => (showEmpty = true)}>
				{t.wallet.emptyFolded(
					emptyGroups.length,
					emptyGroups
						.slice(0, 6)
						.map((g) => `#${g.index}`)
						.join(t.common.comma) + (emptyGroups.length > 6 ? `${t.common.comma}…` : '')
				)}
			</button>
		{:else if showEmpty && emptyGroups.length}
			<button class="ghost folded" onclick={() => (showEmpty = false)}>{t.wallet.hideEmpty}</button>
		{/if}
		{#if skipped.length}
			<p class="small">
				{t.wallet.skipped(skipped[0].index, skipped[skipped.length - 1].index, 10)}
				<button class="ghost small" onclick={() => refresh(skipped)}
					>{t.wallet.checkThem(skipped.length)}</button
				>
			</p>
		{/if}
	</div>
{/if}

<style>
	.empty {
		max-width: 560px;
	}
	ol {
		margin: 0;
		padding-left: 20px;
	}
	.account {
		display: flex;
		align-items: center;
		gap: 14px;
		text-align: left;
		width: 100%;
		font: inherit;
		padding: 16px;
	}
	.folded {
		text-align: left;
		padding: 14px 16px;
		border: 1px dashed var(--stroke);
		border-radius: 14px;
	}
	.account .row {
		flex-wrap: wrap;
		column-gap: 8px;
		row-gap: 0;
	}
	.line {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.account .grow {
		min-width: 0;
	}
	.account:hover {
		border-color: color-mix(in srgb, var(--lamp) 35%, var(--stroke));
	}
</style>
