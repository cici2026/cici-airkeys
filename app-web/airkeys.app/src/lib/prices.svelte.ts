// Prices for the totals, read on demand and kept for a few minutes. Coins and tokens are valued
// by Chainlink feeds, asked through the endpoints in Settings; a currency other than the US
// dollar is converted with Vela's exchange-rate service. Neither request carries an address:
// a feed and a rate table read the same for everyone.
import type { Asset } from './chains/net';
import { getJson, rpcBatch } from './chains/net';
import { chainsOf, store } from './state.svelte';
import { amountOf, FEEDS, LATEST_ROUND, priceKey, readRound } from './value';

export const DEFAULT_FX = 'https://vela-currency.getvela.app/v2/rates?base=USD';
const FRESH_MS = 3 * 60_000;

export const prices = $state({
	/** USD per coin, by feed key. */
	usd: {} as Record<string, number>,
	/** Units of the display currency per US dollar; `null` while unknown. */
	rate: 1 as number | null,
	/** Currencies the rate service offers, USD first. */
	currencies: ['USD'] as string[],
	loading: false,
	/** What could not be read, in words. */
	failed: [] as string[],
	at: 0
});

export const currency = () => store.settings.currency ?? 'USD';

let inFlight: Promise<void> | null = null;

/** Reads the feeds and the rate again, unless they are fresh. Test networks have no prices. */
export function refreshPrices(force = false): Promise<void> {
	if (store.settings.testnet) return Promise.resolve();
	if (!force && Date.now() - prices.at < FRESH_MS) return Promise.resolve();
	inFlight ??= load().finally(() => (inFlight = null));
	return inFlight;
}

async function load() {
	prices.loading = true;
	const failed: string[] = [];
	const usd: Record<string, number> = {};
	const now = Math.floor(Date.now() / 1000);
	// every mainnet EVM chain, whatever the mode: the feeds live on Ethereum and BNB Smart Chain
	const chains = chainsOf('evm', true);
	await Promise.all(
		[...new Set(FEEDS.map((f) => f.chainId))].map(async (id) => {
			const chain = chains.find((c) => c.chainId === id);
			const feeds = FEEDS.filter((f) => f.chainId === id);
			if (!chain) return;
			try {
				const answers = await rpcBatch(
					chain.rpc,
					feeds.map((f) => ['eth_call', [{ to: f.address, data: LATEST_ROUND }, 'latest']])
				);
				feeds.forEach((f, k) => {
					const p = readRound(answers[k], now);
					if (p !== null) usd[f.key] = p;
				});
			} catch (e) {
				failed.push(`prices on ${chain.name}: ${(e as Error).message}`);
			}
		})
	);
	prices.usd = usd;
	prices.rate = await rateFor(currency(), failed);
	prices.failed = failed;
	prices.at = Date.now();
	prices.loading = false;
}

type Row = { quote?: unknown; rate?: unknown };

/** The rate service's table: units of each currency per US dollar. */
async function table(): Promise<Map<string, number>> {
	const rows = await getJson<Row[]>(store.settings.fx || DEFAULT_FX);
	const out = new Map<string, number>([['USD', 1]]);
	for (const r of Array.isArray(rows) ? rows : [])
		if (
			typeof r.quote === 'string' &&
			/^[A-Z]{3}$/.test(r.quote) &&
			typeof r.rate === 'number' &&
			r.rate > 0
		)
			out.set(r.quote, r.rate);
	prices.currencies = ['USD', ...[...out.keys()].filter((q) => q !== 'USD').sort()];
	return out;
}

async function rateFor(code: string, failed: string[]): Promise<number | null> {
	if (code === 'USD') return 1;
	try {
		const rate = (await table()).get(code);
		if (rate === undefined) failed.push(`no exchange rate for ${code}`);
		return rate ?? null;
	} catch (e) {
		failed.push(`the exchange rate for ${code}: ${(e as Error).message}`);
		return null;
	}
}

/** The currencies Settings offers; an error says why there are none. */
export async function loadCurrencies(): Promise<void> {
	if (prices.currencies.length < 2) await table();
}

/** What an asset is worth in the display currency, or `null` (no price, or a test network). */
export function valueOf(a: Asset): number | null {
	if (store.settings.testnet || prices.rate === null) return null;
	const key = priceKey(a);
	const usd = key ? prices.usd[key] : undefined;
	return usd === undefined ? null : amountOf(a) * usd * prices.rate;
}

/** "$1,234.56", "€0.42", "< $0.01". */
export function money(value: number): string {
	const f = new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency: currency(),
		currencyDisplay: 'narrowSymbol'
	});
	return value > 0 && value < 0.01 ? `< ${f.format(0.01)}` : f.format(value);
}
