<script lang="ts">
	// Endpoints per chain, and EVM chains added by hand. Everything stays in this browser.
	// Self-hosters can point every chain at their own nodes.
	import { registry, type Chain, type Token } from '$lib/registry';
	import { save, store } from '$lib/state.svelte';
	import { isHttpUrl, rpc } from '$lib/chains/net';
	import { DEFAULT_FX, loadCurrencies, prices, refreshPrices } from '$lib/prices.svelte';
	import {
		CHAIN_DATA,
		chainDetails,
		chainIndex,
		searchChains,
		type ChainHit,
		type ChainOffer
	} from '$lib/chainData';

	let problem = $state('');
	let fxProblem = $state('');
	let fxText = $state(store.settings.fx ?? '');

	async function currencies() {
		fxProblem = '';
		try {
			await loadCurrencies();
		} catch (e) {
			fxProblem = `The currencies did not load: ${(e as Error).message}. Totals stay in US dollars.`;
		}
	}
	function setCurrency(code: string) {
		if (code === 'USD') delete store.settings.currency;
		else store.settings.currency = code;
		save();
		void refreshPrices(true);
	}
	function setFx() {
		const url = fxText.trim();
		if (url && !isHttpUrl(url)) {
			fxProblem = 'Enter a web address: http://… or https://…';
			return;
		}
		fxProblem = '';
		if (url) store.settings.fx = url;
		else delete store.settings.fx;
		save();
		prices.currencies = ['USD'];
		void refreshPrices(true);
	}
	let added = $state('');
	/** Endpoint rows open for editing, and what is wrong with what was typed in each. */
	let editing = $state<Record<string, boolean>>({});
	let wrong = $state<Record<string, string>>({});
	let custom = $state({ name: '', chainId: '', symbol: '', rpc: '', explorer: '' });

	// adding a chain: search the chains of ethereum-data.getvela.app, or enter one by hand
	let query = $state('');
	let all = $state<ChainHit[]>([]);
	let searching = $state('');
	let offer = $state<ChainOffer | null>(null);
	let picked = $state<Record<string, boolean>>({});
	let loadingOffer = $state(false);
	let byHand = $state(false);
	const known = (id: number) =>
		[...registry.evm, ...store.settings.chains].some((c) => c.chainId === id);
	const hits = $derived(searchChains(all, query));

	async function startSearch() {
		if (all.length || searching === 'loading') return;
		searching = 'loading';
		try {
			all = await chainIndex();
			searching = '';
		} catch {
			searching = `The chain list at ${new URL(CHAIN_DATA).host} did not load. Enter the chain by hand below.`;
		}
	}

	async function pick(hit: ChainHit) {
		problem = added = '';
		offer = null;
		loadingOffer = true;
		try {
			offer = await chainDetails(hit.chainId);
			picked = Object.fromEntries(offer.tokens.map((t) => [t.address, true]));
		} catch (e) {
			problem = (e as Error).message;
		} finally {
			loadingOffer = false;
		}
	}

	function addOffer() {
		if (!offer) return;
		const tokens: Token[] = offer.tokens.filter((t) => picked[t.address]);
		store.settings.chains.push({ ...offer.chain, tokens });
		save();
		added = `Added ${offer.chain.name}${offer.chain.testnet ? ' (a testnet: switch to Testnet to see it)' : ''}. The phone signs for it, and warns that it is not in its own list.`;
		offer = null;
		query = '';
	}

	type Row = { key: string; name: string; chain: Chain; kind: string };
	const rows = $derived<Row[]>([
		{ key: 'bitcoin', name: 'Bitcoin', chain: registry.bitcoin, kind: 'Esplora API' },
		...[...registry.evm, ...store.settings.chains].map((c) => ({
			key: `evm:${c.chainId}`,
			name: c.name,
			chain: c,
			kind: 'JSON-RPC'
		}))
	]);
	const groups = $derived([
		{ title: 'Mainnet', rows: rows.filter((r) => !r.chain.testnet) },
		{ title: 'Testnet', rows: rows.filter((r) => r.chain.testnet) }
	]);

	/** A web page's host, or nothing: the data comes from a third party. */
	const host = (url: string) => {
		try {
			return new URL(url).host;
		} catch {
			return '';
		}
	};

	function set(key: string, value: string) {
		const url = value.trim();
		if (url && !isHttpUrl(url)) {
			wrong[key] = 'Enter a web address: http://… or https://…';
			return;
		}
		delete wrong[key];
		if (url) store.settings.rpc[key] = url;
		else delete store.settings.rpc[key];
		save();
	}

	async function addChain() {
		problem = added = '';
		try {
			const id = Number(custom.chainId);
			if (!Number.isInteger(id) || id <= 0) throw new Error('Enter the chain id.');
			if ([...registry.evm, ...store.settings.chains].some((c) => c.chainId === id))
				throw new Error('This chain is already listed.');
			const endpoint = custom.rpc.trim();
			if (!isHttpUrl(endpoint))
				throw new Error('Enter the RPC URL, a web address: http://… or https://…');
			const ex = custom.explorer.trim().replace(/\/$/, '');
			if (ex && !isHttpUrl(ex))
				throw new Error('Enter the explorer as a web address (https://…), or leave it empty.');
			const answered = Number(BigInt(await rpc<string>(endpoint, 'eth_chainId')));
			if (answered !== id) throw new Error(`That endpoint serves chain ${answered}, not ${id}.`);
			store.settings.chains.push({
				chainId: id,
				name: custom.name || `Chain ${id}`,
				symbol: custom.symbol || 'ETH',
				decimals: 18,
				rpc: endpoint,
				explorer: ex ? { tx: `${ex}/tx/{}`, address: `${ex}/address/{}` } : { tx: '', address: '' },
				tokens: []
			});
			save();
			added = `Added ${custom.name || `chain ${id}`}. The phone signs for it, and warns that it is not in its list.`;
			custom = { name: '', chainId: '', symbol: '', rpc: '', explorer: '' };
		} catch (e) {
			problem = (e as Error).message;
		}
	}

	function removeChain(c: Chain) {
		const key = `evm:${c.chainId}`;
		store.settings.chains = store.settings.chains.filter((x) => x.chainId !== c.chainId);
		delete store.settings.rpc[key];
		delete editing[key];
		delete wrong[key];
		save();
	}
</script>

<div class="stack narrow">
	<h2>Settings</h2>
	<section class="card stack">
		<div class="label">Networks</div>
		<input
			class="search"
			bind:value={query}
			onfocus={startSearch}
			placeholder="Search a network — name, symbol or chain id"
			aria-label="Search a network"
			spellcheck="false"
			autocomplete="off"
		/>
		{#if searching && searching !== 'loading'}<div class="notice warn small" role="status">
				{searching}
			</div>{/if}
		{#if query && hits.length}
			<div class="hits">
				{#each hits as h (h.chainId)}
					<button class="hit" disabled={known(h.chainId)} onclick={() => pick(h)}>
						<span class="grow"
							><b>{h.name}</b> <span class="small">· {h.symbol} · chain {h.chainId}</span></span
						>
						{#if h.testnet}<span class="pill warn">Testnet</span>{/if}
						{#if known(h.chainId)}<span class="small">Listed</span>{/if}
					</button>
				{/each}
			</div>
		{:else if query && all.length}
			<p class="small">No network matches “{query}”.</p>
		{/if}
		{#if loadingOffer}<p class="small" role="status">Checking its endpoints…</p>{/if}
		{#if offer}
			<div class="offer stack">
				<div class="row">
					<b class="grow">{offer.chain.name}</b>
					{#if offer.chain.testnet}<span class="pill warn">Testnet</span>{:else}<span class="pill"
							>Mainnet</span
						>{/if}
				</div>
				<div class="small">
					Chain {offer.chain.chainId} · {offer.chain.symbol} · endpoint
					<span class="mono">{offer.chain.rpc}</span>
					(it answered for this chain){host(offer.chain.explorer.tx)
						? ` · explorer ${host(offer.chain.explorer.tx)}`
						: ''}
				</div>
				{#if offer.tokens.length}
					<div class="small">Tokens (decimals read from the chain):</div>
					{#each offer.tokens as t (t.address)}
						<label class="row token">
							<input type="checkbox" bind:checked={picked[t.address]} />
							<b>{t.symbol}</b><span class="mono small grow">{t.address}</span>
						</label>
					{/each}
				{/if}
				<div><button class="primary" onclick={addOffer}>Add {offer.chain.name}</button></div>
			</div>
		{/if}
		<p class="small">
			Networks come from <a href={CHAIN_DATA} target="_blank" rel="noreferrer"
				>{new URL(CHAIN_DATA).host}</a
			>.
			<button class="ghost small" aria-expanded={byHand} onclick={() => (byHand = !byHand)}
				>{byHand ? 'Hide' : 'Enter one by hand'}</button
			>
		</p>
		{#if byHand}
			<div class="grid">
				<input bind:value={custom.name} placeholder="Name" aria-label="Name" />
				<input
					bind:value={custom.chainId}
					placeholder="Chain id"
					aria-label="Chain id"
					inputmode="numeric"
				/>
				<input bind:value={custom.symbol} placeholder="Coin symbol" aria-label="Coin symbol" />
				<input bind:value={custom.rpc} placeholder="RPC URL" aria-label="RPC URL" />
				<input
					class="wide"
					bind:value={custom.explorer}
					placeholder="Explorer URL (https://…)"
					aria-label="Explorer URL"
				/>
			</div>
			<div><button class="primary" onclick={addChain}>Add chain</button></div>
		{/if}
		{#if problem}<div class="notice danger" role="alert">{problem}</div>{/if}
		{#if added}<div class="notice info" role="status">{added}</div>{/if}
		{#each groups as g (g.title)}
			{#if g.rows.length}
				<div class="group">{g.title}</div>
				{#each g.rows as r (r.key)}
					<div class="net">
						<div class="row">
							<b class="grow">{r.name}</b>
							{#if store.settings.rpc[r.key]}<span class="pill dim">Own endpoint</span>{/if}
							<button
								class="ghost small"
								aria-expanded={!!editing[r.key]}
								onclick={() => (editing[r.key] = !editing[r.key])}>Endpoint</button
							>
							{#if store.settings.chains.includes(r.chain)}<button
									class="ghost small"
									onclick={() => removeChain(r.chain)}>Remove</button
								>{/if}
						</div>
						{#if editing[r.key]}
							<input
								value={store.settings.rpc[r.key] ?? ''}
								placeholder={`${r.kind}: ${r.chain.rpc}`}
								aria-label={`${r.name} endpoint (${r.kind})`}
								onchange={(e) => set(r.key, e.currentTarget.value)}
								spellcheck="false"
							/>
							{#if wrong[r.key]}<div class="notice danger small" role="alert">
									{wrong[r.key]}
								</div>{/if}
						{/if}
					</div>
				{/each}
			{/if}
		{/each}
		<p class="small">
			Airkeys reads balances and broadcasts through these endpoints. Point them at your own nodes
			for privacy; leave one empty for the default.
		</p>
	</section>
	<section class="card stack">
		<div class="label">Prices</div>
		<label class="row">
			<span class="grow">Show totals in</span>
			<select
				class="currency"
				value={store.settings.currency ?? 'USD'}
				onfocus={currencies}
				onchange={(e) => setCurrency(e.currentTarget.value)}
			>
				{#each prices.currencies.includes(store.settings.currency ?? 'USD') ? prices.currencies : [store.settings.currency ?? 'USD', ...prices.currencies] as c (c)}
					<option value={c}>{c}</option>
				{/each}
			</select>
		</label>
		<p class="small">
			Coins and tokens are valued by Chainlink price feeds, read through the Ethereum and BNB Smart
			Chain endpoints above; only what Airkeys lists is priced. Other currencies are converted with
			the rates of <span class="mono">{new URL(store.settings.fx || DEFAULT_FX).host}</span>.
			Neither carries an address.
		</p>
		<label class="stack tight">
			<span class="small">Exchange-rate endpoint (rates from USD)</span>
			<input bind:value={fxText} onchange={setFx} placeholder={DEFAULT_FX} spellcheck="false" />
		</label>
		{#if fxProblem}<div class="notice warn small" role="status">{fxProblem}</div>{/if}
	</section>
	<section class="card stack">
		<div class="label">This browser</div>
		<p class="small">
			The web app keeps only addresses and these settings, in this browser. It never sees a key.
		</p>
		<div>
			<button
				class="danger"
				onclick={() => {
					if (
						confirm(
							'Forget every account in this browser? The funds are untouched; connect the phone again to see them.'
						)
					) {
						store.accounts = [];
						save();
					}
				}}>Forget all accounts</button
			>
		</div>
	</section>
</div>

<style>
	.currency {
		width: auto;
		min-width: 110px;
	}
	.tight {
		gap: 6px;
	}
	.narrow {
		max-width: 680px;
	}
	.grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}
	.wide {
		grid-column: span 2;
	}
	.group {
		margin-top: 8px;
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--faint);
	}
	.net {
		display: grid;
		gap: 6px;
		padding: 4px 0;
		border-top: 1px solid var(--stroke);
	}
	.net .row {
		min-height: 40px;
	}
	.hits {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.hit {
		display: flex;
		align-items: center;
		gap: 10px;
		text-align: left;
		font: inherit;
		padding: 10px 12px;
	}
	.offer {
		padding: 14px;
		border-radius: 12px;
		background: var(--raised);
	}
	.token {
		gap: 10px;
		align-items: center;
	}
</style>
