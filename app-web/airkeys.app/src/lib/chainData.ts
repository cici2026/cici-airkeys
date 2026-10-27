// EVM chains from ethereum-data.getvela.app (github.com/atshelchin/ethereum-data): a searchable
// list of every chain, and for each its endpoints, explorers and stablecoins. Used to add a
// chain without typing its details; nothing from it is trusted blindly — the endpoint must
// answer with the chain id asked for, and token decimals are read from the chain itself.
import type { Chain, Token } from './registry';
import { getJson, isHttpUrl, rpc } from './chains/net';
import { parseEvm } from './addresses';
import { t } from './i18n/index.svelte';

export const CHAIN_DATA = 'https://ethereum-data.getvela.app';

export interface ChainHit {
	chainId: number;
	name: string;
	shortName: string;
	symbol: string;
	testnet: boolean;
}

/** A test network: no value. The data has no flag for it, so the usual signs are read. */
const looksTest = (text: string) =>
	/test|sepolia|holesky|hoodi|goerli|devnet|ropsten|rinkeby|kovan/i.test(text);

let index: Promise<ChainHit[]> | null = null;

/** Every chain, fetched once per page load. */
export function chainIndex(): Promise<ChainHit[]> {
	index ??= getJson<{
		data: { chainId: number; name: string; shortName: string; nativeCurrencySymbol: string }[];
	}>(`${CHAIN_DATA}/index/fuse-chains.json`)
		.then((r) =>
			r.data.map((c) => ({
				chainId: c.chainId,
				name: c.name,
				shortName: c.shortName,
				symbol: c.nativeCurrencySymbol,
				testnet: looksTest(c.name)
			}))
		)
		.catch((e) => {
			index = null;
			throw e;
		});
	return index;
}

/** The best matches for `query`: a chain id exactly, then names and symbols. */
export function searchChains(list: ChainHit[], query: string, max = 8): ChainHit[] {
	const q = query.trim().toLowerCase();
	if (!q) return [];
	const score = (c: ChainHit) => {
		const name = c.name.toLowerCase();
		if (String(c.chainId) === q) return 0;
		if (name === q || c.shortName.toLowerCase() === q) return 1;
		if (name.startsWith(q)) return 2;
		if (name.split(/\s+/).some((w) => w.startsWith(q))) return 3;
		if (c.symbol.toLowerCase() === q) return 4;
		if (name.includes(q)) return 5;
		return 99;
	};
	return list
		.map((c) => [score(c), c] as const)
		.filter(([s]) => s < 99)
		.sort(
			(a, b) =>
				a[0] - b[0] || Number(a[1].testnet) - Number(b[1].testnet) || a[1].chainId - b[1].chainId
		)
		.slice(0, max)
		.map(([, c]) => c);
}

interface ChainRecord {
	name: string;
	title?: string;
	chainId: number;
	rpc: string[];
	nativeCurrency: { symbol: string; decimals: number };
	explorers?: { url: string; standard?: string }[];
	faucets?: string[];
	slip44?: number;
	stables?: { symbol: string; contract: string }[];
}

export interface ChainOffer {
	chain: Chain;
	/** Stablecoins the data lists for it, decimals read from the chain. */
	tokens: Token[];
}

/** How long an endpoint has to answer for its chain id. */
const PROBE_MS = 5000;

/** One chain's details, with an endpoint that answered for this chain id. */
export async function chainDetails(chainId: number): Promise<ChainOffer> {
	const r = await getJson<ChainRecord>(`${CHAIN_DATA}/chains/eip155-${chainId}.json`);
	if (r.chainId !== chainId) throw new Error(t.chainData.mismatch);
	if (
		typeof r.name !== 'string' ||
		typeof r.nativeCurrency?.symbol !== 'string' ||
		!Number.isInteger(r.nativeCurrency.decimals) ||
		!Array.isArray(r.rpc)
	)
		throw new Error(t.chainData.invalid);
	// plain HTTPS endpoints without API keys, all asked at once: the first to serve this chain
	const candidates = r.rpc
		.filter((u) => typeof u === 'string' && u.startsWith('https://') && !u.includes('${'))
		.filter(isHttpUrl)
		.slice(0, 6);
	let endpoint: string;
	try {
		endpoint = await Promise.any(
			candidates.map(async (url) => {
				const id = await Promise.race([
					rpc<string>(url, 'eth_chainId'),
					new Promise<never>((_, no) => setTimeout(() => no(new Error('no answer')), PROBE_MS))
				]);
				if (Number(BigInt(id)) !== chainId) throw new Error('another chain');
				return url;
			})
		);
	} catch {
		throw new Error(t.chainData.noEndpoint);
	}
	const site = (
		r.explorers?.find((e) => e.standard === 'EIP3091') ?? r.explorers?.[0]
	)?.url?.replace(/\/$/, '');
	// a link only to a web address
	const explorer = site && isHttpUrl(site) ? site : '';
	const chain: Chain = {
		chainId,
		name: r.name,
		symbol: r.nativeCurrency.symbol,
		decimals: r.nativeCurrency.decimals,
		rpc: endpoint,
		explorer: explorer
			? { tx: `${explorer}/tx/{}`, address: `${explorer}/address/{}` }
			: { tx: '', address: '' },
		tokens: [],
		testnet:
			r.slip44 === 1 || (r.faucets?.length ?? 0) > 0 || looksTest(`${r.name} ${r.title ?? ''}`)
	};
	const tokens: Token[] = [];
	for (const s of (r.stables ?? []).slice(0, 8)) {
		try {
			// decimals(): read from the token contract, never taken from the list
			const d = Number(
				BigInt(
					await rpc<string>(endpoint, 'eth_call', [
						{ to: s.contract, data: '0x313ce567' },
						'latest'
					])
				)
			);
			if (Number.isInteger(d) && d >= 0 && d <= 36)
				tokens.push({ address: parseEvm(s.contract), symbol: s.symbol, decimals: d });
		} catch {
			/* not readable: left out */
		}
	}
	return { chain, tokens };
}
