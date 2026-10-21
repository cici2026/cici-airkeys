<script lang="ts">
	// A transfer, end to end: the form, the request shown to the phone, the phone's signed reply,
	// and broadcasting. The phone checks everything again and shows it before it signs.
	import { broadcast, prepare, type Asset, type Broadcast, type Prepared } from '$lib/chains';
	import { encode, parseMessage, type Account, type Signed } from '$lib/protocol';
	import { explorerLink, NETWORK_NAMES } from '$lib/registry';
	import { addAccounts, store } from '$lib/state.svelte';
	import { ensure, freshIndex, holdingOf, refresh } from '$lib/holdings.svelte';
	import { parseAddress } from '$lib/addresses';
	import { formatUnits, parseUnits } from '$lib/units';
	import CodeCanvas from '$lib/lumen/CodeCanvas.svelte';
	import CameraScan from '$lib/lumen/CameraScan.svelte';
	import { untrack } from 'svelte';

	let { account, asset, back }: { account: Account; asset: Asset; back: () => void } = $props();

	type Step = 'form' | 'review' | 'request' | 'reply' | 'done';
	let step = $state<Step>('form');
	let to = $state('');
	let amountText = $state('');
	let max = $state(false);
	let speed = $state<0 | 1 | 2>(1);
	const exposure = $derived(holdingOf(account)?.exposure);
	// Bitcoin change always goes to a fresh account; on EVM chains moving the rest is the
	// default: the first send reveals the key
	let moveRest = $state(true);
	// the lowest connected account seen unused — recomputed as balances arrive, until the person
	// types another number
	let typed = $state<number | null>(null);
	const fresh = $derived(typed ?? freshIndex(account.network, account.index));
	$effect(() => {
		// read every connected account, so the fresh one can be found
		ensure(...store.accounts.filter((a) => a.network === account.network));
	});
	let problem = $state('');
	let busy = $state(false);
	let prepared = $state<Prepared | null>(null);
	let results = $state<Broadcast[]>([]);

	const holdings = $derived(
		(holdingOf(account)?.assets ?? []).filter(
			// one chain: by chain id (a chain added by hand may share a listed chain's name)
			(a) => a.chain.chainId === asset.chain.chainId && a.chain.name === asset.chain.name
		)
	);

	async function review() {
		problem = '';
		try {
			const recipient = parseAddress(account.network, to);
			const amount = max ? null : parseUnits(amountText, asset.decimals);
			if (!max && (amount === null || amount <= 0n)) throw new Error('Enter an amount.');
			if ((moveRest || account.network === 'bitcoin') && fresh === null)
				throw new Error(
					'Airkeys has not seen an unused account to move the rest to. On the phone, Connect → All 100 accounts, then scan it here.'
				);
			if (moveRest && fresh === account.index)
				throw new Error('Move the rest to another account number.');
			busy = true;
			prepared = await prepare(account.network, {
				from: account,
				asset,
				to: recipient,
				amount,
				moveRest: moveRest ? fresh : null,
				holdings,
				speed
			});
			step = 'review';
		} catch (e) {
			problem = (e as Error).message;
		} finally {
			busy = false;
		}
	}

	async function replied(bytes: Uint8Array) {
		problem = '';
		let signed: Signed;
		try {
			const m = parseMessage(bytes);
			if (m.type !== 'signed') throw new Error('This code holds accounts, not a signed reply.');
			if (m.signed.id !== prepared!.request.id)
				throw new Error('This reply answers another request. Show the phone this request again.');
			signed = m.signed;
		} catch (e) {
			problem = (e as Error).message;
			return;
		}
		busy = true;
		step = 'done';
		results = await broadcast(
			account.network,
			asset.chain,
			signed.txs.map((t) => t.raw)
		);
		// a reply can name accounts the request used by number: now the web app knows them
		addAccounts(signed.accounts);
		busy = false;
		setTimeout(() => refresh([account]), 4000);
	}
</script>

<div class="stack narrow">
	<div class="row">
		<button class="ghost" onclick={back}>← {NETWORK_NAMES[account.network]} #{account.index}</button
		>
	</div>
	<h2>Send {asset.symbol}{account.network === 'evm' ? ` on ${asset.chain.name}` : ''}</h2>

	{#if step === 'form'}
		<div class="card stack">
			<label class="stack tight">
				<span class="label">To</span>
				<input
					bind:value={to}
					placeholder={account.network === 'evm' ? '0x…' : 'Address'}
					autocomplete="off"
					spellcheck="false"
				/>
			</label>
			<label class="stack tight">
				<span class="label">Amount</span>
				<div class="row">
					<input
						class="grow"
						bind:value={amountText}
						placeholder="0.0"
						inputmode="decimal"
						disabled={max}
					/>
					<button class:primary={max} onclick={() => (max = !max)}>Max</button>
				</div>
				<span class="small"
					>Available: {formatUnits(asset.amount, asset.decimals)} {asset.symbol}</span
				>
			</label>
			<div class="stack tight">
				<span class="label">Network fee</span>
				<div class="row speeds">
					{#each ['Slow', 'Normal', 'Fast'] as s, i (s)}
						<button class:primary={speed === i} onclick={() => (speed = i as 0 | 1 | 2)}>{s}</button
						>
					{/each}
				</div>
			</div>
			{#if account.network === 'bitcoin'}
				<div class="stack tight">
					<span class="label">Change</span>
					<p class="small">
						A Bitcoin transaction spends whole coins, so every coin of this account is spent. What
						is not sent comes back as change to account #<input
							class="index"
							type="number"
							min="0"
							value={fresh ?? ''}
							oninput={(e) =>
								(typed = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
							aria-label="Account for the change"
						/>, whose key is still hidden.
					</p>
				</div>
			{:else}
				<div class="stack tight" role="radiogroup" aria-label="The rest of this account">
					<span class="label">The rest of this account</span>
					<label class="choice" class:on={moveRest}>
						<input type="radio" name="rest" value={true} bind:group={moveRest} />
						<span class="grow">
							<b
								>Move it to account #<input
									class="index"
									type="number"
									min="0"
									value={fresh ?? ''}
									oninput={(e) =>
										(typed = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}
									aria-label="Fresh account number"
								/></b
							>
							<span class="pill safe">Recommended</span>
							<span class="small"
								>This send reveals the account's public key. Everything else it holds on {asset
									.chain.name} moves, in the same signing, to an account whose key is still hidden.</span
							>
						</span>
					</label>
					<label class="choice" class:on={!moveRest}>
						<input type="radio" name="rest" value={false} bind:group={moveRest} />
						<span class="grow">
							<b>Leave it here</b>
							<span class="small"
								>Only the amount above is sent. The rest stays at this address, whose public key
								this send reveals.</span
							>
						</span>
					</label>
				</div>
			{/if}
			{#if exposure === 'exposed'}<div class="notice warn small">
					This account's key is already exposed. Moving everything to a fresh account is the safe
					choice.
				</div>{/if}
			{#if problem}<div class="notice danger">{problem}</div>{/if}
			<button class="primary" onclick={review} disabled={busy}
				>{busy ? 'Preparing…' : 'Review'}</button
			>
		</div>
	{:else if step === 'review' && prepared}
		<div class="card stack">
			<div class="label">The phone will sign</div>
			<ol class="lines">
				{#each prepared.summary as line, i (i)}<li class="mono">{line}</li>{/each}
			</ol>
			<div class="row"><span class="dim grow">Network fee, at most</span><b>{prepared.fee}</b></div>
			<div class="row">
				<button onclick={() => (step = 'form')}>Edit</button>
				<button class="primary grow" onclick={() => (step = 'request')}>Show to the phone</button>
			</div>
		</div>
	{:else if step === 'request' && prepared}
		<p class="dim">
			On the phone: <b>Scan request</b>, point it at this code, check every line, then hold to sign.
		</p>
		<CodeCanvas data={encode(prepared.request)} name="airkeys-sign.json" />
		<div class="row">
			<button onclick={() => (step = 'review')}>Back</button>
			<button class="primary grow" onclick={() => (step = 'reply')}>Scan the phone's reply</button>
		</div>
		<details class="small">
			<summary>What this code says</summary>
			<pre>{JSON.stringify(prepared.request, null, 2)}</pre>
		</details>
	{:else if step === 'reply'}
		<p class="dim">On the phone, after signing: hold its code in front of this camera.</p>
		<CameraScan onmessage={replied} />
		{#if problem}<div class="notice danger">{problem}</div>{/if}
		<div><button onclick={() => (step = 'request')}>Show the request again</button></div>
	{:else if step === 'done'}
		<div class="card stack">
			{#if busy}
				<p>Broadcasting…</p>
			{:else}
				{#each results as r, i (i)}
					<div class="row">
						<span class="pill {r.ok ? 'safe' : 'warn'}">{r.ok ? 'Sent' : 'Refused'}</span>
						{#if r.hash}<a
								class="mono small grow hash"
								href={explorerLink(asset.chain.explorer.tx, r.hash)}
								target="_blank"
								rel="noreferrer">{r.hash}</a
							>{/if}
						{#if r.error}<span class="small grow">{r.error}</span>{/if}
					</div>
				{/each}
				<button class="primary" onclick={back}>Done</button>
			{/if}
		</div>
	{/if}
</div>

<style>
	.narrow {
		max-width: 620px;
	}
	.tight {
		gap: 6px;
	}
	.speeds button {
		flex: 1;
	}
	.choice {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 12px 14px;
		border: 1px solid var(--stroke);
		border-radius: 12px;
		cursor: pointer;
	}
	.choice.on {
		border-color: var(--lamp);
		background: color-mix(in srgb, var(--lamp) 7%, var(--raised));
	}
	.choice input[type='radio'] {
		width: 18px;
		height: 18px;
		margin-top: 3px;
		accent-color: var(--lamp);
	}
	.choice .small {
		display: block;
		margin-top: 3px;
	}
	.choice .pill {
		margin-left: 6px;
		vertical-align: 2px;
	}
	.index {
		display: inline-block;
		width: 72px;
		padding: 2px 6px;
		margin: 0 2px;
	}
	.lines {
		margin: 0;
		padding-left: 22px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		word-break: break-all;
		font-size: 14px;
	}
	.hash {
		word-break: break-all;
	}
	pre {
		white-space: pre-wrap;
		word-break: break-all;
		color: var(--dim);
	}
</style>
