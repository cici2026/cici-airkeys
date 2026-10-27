<script lang="ts">
	// One account on one network: where to receive, what it holds, whether its key is hidden.
	import { NETWORK_NAMES, explorerLink, path } from '#lib/registry.ts';
	import { chainsOf, removeAccount } from '#lib/state.svelte.ts';
	import { holdingOf, refresh, ensure } from '#lib/holdings.svelte.ts';
	import { formatUnits } from '#lib/units.ts';
	import type { Account } from '#lib/protocol.ts';
	import type { Asset } from '#lib/chains/index.ts';
	import Address from '#lib/ui/Address.svelte';
	import Badge from '#lib/ui/Badge.svelte';
	import Qr from '#lib/ui/Qr.svelte';
	import Exposure from './Exposure.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';

	let {
		account,
		send,
		move,
		back
	}: {
		account: Account;
		send: (a: Asset) => void;
		move: (a: Asset) => void;
		back: () => void;
	} = $props();
	const h = $derived(holdingOf(account));
	// none on a network without a chain in this mode, or with an explorer that is not a web page
	const explorer = $derived(
		explorerLink(chainsOf(account.network)[0]?.explorer.address ?? '', account.address)
	);
	let copied = $state('');
	// a skipped account (past the gap of unused ones) is read when it is opened
	$effect(() => {
		if (holdingOf(account)?.skipped) refresh([account]);
		else ensure(account);
	});

	// chains holding something get their own list; the empty ones share one line
	const shown = $derived((h?.assets ?? []).filter((a) => a.amount > 0n));
	const chains = $derived([...new Set(shown.map((a) => a.chain.name))]);
	const empty = $derived(
		[...new Set((h?.assets ?? []).map((a) => a.chain.name))].filter((c) => !chains.includes(c))
	);

	async function copy() {
		try {
			await navigator.clipboard.writeText(account.address);
			copied = t.account.copied;
		} catch {
			// no clipboard (an insecure page, or not allowed): the address is on screen
			copied = t.account.notCopied;
		}
		setTimeout(() => (copied = ''), 1500);
	}
</script>

<div class="stack">
	<div class="row">
		<button class="ghost" onclick={back}>← {t.common.home}</button>
		<div class="grow"></div>
		<button class="ghost" onclick={() => refresh([account])} disabled={h?.loading}
			>{h?.loading ? t.account.refreshing : t.common.refresh}</button
		>
	</div>
	<div class="row">
		<Badge network={account.network} size={52} />
		<div class="grow">
			<h2>{t.account.title(NETWORK_NAMES[account.network], account.index)}</h2>
			<div class="small mono">{path(account.network, account.index)}</div>
		</div>
		<Exposure exposure={h?.exposure ?? 'unknown'} loading={!h || h.loading} />
	</div>

	<div class="receive card">
		<Qr value={account.address} size={168} />
		<div class="stack grow">
			<div class="label">{t.account.receiveAt}</div>
			<Address address={account.address} size={17} />
			<div class="row">
				<button onclick={copy}>{copied || t.account.copyAddress}</button>
				{#if explorer}<a class="small" href={explorer} target="_blank" rel="noreferrer"
						>{t.account.explorer}</a
					>{/if}
			</div>
			{#if account.network === 'evm'}<p class="small">{t.account.sameOnEvm}</p>{/if}
		</div>
	</div>

	{#if h?.exposure === 'exposed'}
		<div class="notice warn">
			{t.account.exposedBefore}<b>{t.account.moveAll}</b>{t.account.exposedAfter}
		</div>
	{:else if h?.exposure === 'hidden'}
		<div class="notice info">
			{t.account.hidden}
		</div>
	{/if}

	{#each chains as c (c)}
		<section class="stack">
			{#if account.network === 'evm' || h?.exposure === 'exposed'}
				<div class="row">
					<div class="label grow">{account.network === 'evm' ? c : ''}</div>
					{#if h?.exposure === 'exposed'}
						<button
							class="small"
							onclick={() =>
								move(
									shown.find((a) => a.chain.name === c && !a.token) ??
										shown.find((a) => a.chain.name === c)!
								)}>{t.account.moveAll}</button
						>
					{/if}
				</div>
			{/if}
			{#each shown.filter((a) => a.chain.name === c) as a (a.key)}
				<div class="asset card row">
					<div class="grow">
						<div class="amount">
							{formatUnits(a.amount, a.decimals)} <span class="dim">{a.symbol}</span>
						</div>
						{#if a.token}<div class="small mono">{a.token.address}</div>{/if}
					</div>
					<button class="primary" onclick={() => send(a)}>{t.account.send}</button>
				</div>
			{/each}
		</section>
	{/each}
	{#if !chains.length && h && !h.loading && !h.failed.length}
		<div class="card stack empty">
			<b>{t.account.nothingToSend}</b>
			<span class="small"
				>{t.account.holdsNothingBefore(empty.join(t.common.comma))}<b>{t.account.send}</b>{t.account
					.holdsNothingAfter}</span
			>
		</div>
	{:else if chains.length && empty.length && !h?.loading}
		<p class="small">{t.account.nothingOn(empty.join(t.common.comma))}</p>
	{/if}
	{#if h?.failed.length}
		<div class="notice warn small" role="status">
			{t.account.unreachable(h.failed.join(t.common.semicolon))}
		</div>
	{/if}
	<div>
		<button class="ghost small" onclick={() => (removeAccount(account), back())}
			>{t.account.remove}</button
		>
	</div>
</div>

<style>
	.receive {
		display: flex;
		gap: 22px;
		align-items: center;
		flex-wrap: wrap;
	}
	/* beside the code while there is room for a whole address line, below it otherwise */
	.receive > .stack {
		flex: 1 1 260px;
	}
	@media (max-width: 560px) {
		h2 {
			font-size: 20px;
		}
	}
	.amount {
		font-size: 20px;
		font-weight: 650;
		font-variant-numeric: tabular-nums;
	}
	.asset .mono {
		word-break: break-all;
	}
</style>
