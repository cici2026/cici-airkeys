// What the web app keeps: the accounts the phone has shown it (addresses only) and the
// settings (endpoints, extra EVM chains). Both live in this browser's storage and nowhere else;
// nothing here is secret.
import { NETWORKS, registry, type Chain, type Network, type Token } from './registry';
import { isIndex, type Account } from './protocol';
import { isHttpUrl } from './chains/net';

export interface Settings {
	/** Endpoint per chain key: "bitcoin", "evm:<chainId>". */
	rpc: Record<string, string>;
	/** EVM chains added by hand. */
	chains: Chain[];
	/** Show test networks instead of the real ones: their coins have no value, and the two are
	 * never shown together. */
	testnet?: boolean;
}

const KEY = 'airkeys.v1';

const isObject = (v: unknown): v is Record<string, unknown> =>
	typeof v === 'object' && v !== null && !Array.isArray(v);

/** A stored chain, or `null` when it is not one: storage is anyone's to edit. */
function storedChain(c: unknown): Chain | null {
	if (!isObject(c)) return null;
	const { chainId, name, symbol, decimals, rpc, explorer, tokens, testnet } = c;
	if (
		!Number.isInteger(chainId) ||
		typeof name !== 'string' ||
		typeof symbol !== 'string' ||
		!Number.isInteger(decimals) ||
		typeof rpc !== 'string' ||
		!isHttpUrl(rpc)
	)
		return null;
	const text = (v: unknown) => (typeof v === 'string' ? v : '');
	return {
		chainId: chainId as number,
		name,
		symbol,
		decimals: decimals as number,
		rpc,
		explorer: isObject(explorer)
			? { tx: text(explorer.tx), address: text(explorer.address) }
			: { tx: '', address: '' },
		tokens: (Array.isArray(tokens) ? tokens : []).filter(
			(t): t is Token =>
				isObject(t) &&
				typeof t.address === 'string' &&
				typeof t.symbol === 'string' &&
				Number.isInteger(t.decimals)
		),
		...(testnet === true ? { testnet } : {})
	};
}

function load(): { accounts: Account[]; settings: Settings } {
	const empty = { accounts: [], settings: { rpc: {}, chains: [] } };
	try {
		const v = JSON.parse(localStorage.getItem(KEY) ?? 'null');
		if (!isObject(v) || !Array.isArray(v.accounts)) return empty;
		const s = isObject(v.settings) ? v.settings : {};
		return {
			// accounts of unknown networks are dropped
			accounts: v.accounts.filter(
				(a: unknown): a is Account =>
					isObject(a) &&
					NETWORKS.includes(a.network as Network) &&
					isIndex(a.index) &&
					typeof a.address === 'string'
			),
			// what older versions kept too (explorers per chain) is left out
			settings: {
				rpc: Object.fromEntries(
					Object.entries(isObject(s.rpc) ? s.rpc : {}).filter(
						(e): e is [string, string] => typeof e[1] === 'string' && isHttpUrl(e[1])
					)
				),
				chains: (Array.isArray(s.chains) ? s.chains : [])
					.map(storedChain)
					.filter((c): c is Chain => !!c),
				...(typeof s.testnet === 'boolean' ? { testnet: s.testnet } : {})
			}
		};
	} catch {
		return empty;
	}
}

export const store = $state(
	typeof localStorage === 'undefined'
		? { accounts: [] as Account[], settings: { rpc: {}, chains: [] } as Settings }
		: load()
);

export function save() {
	try {
		localStorage.setItem(KEY, JSON.stringify(store));
	} catch {
		/* private mode: the session still works */
	}
}

export function chainKey(network: Network, chain?: Chain): string {
	return network === 'evm' ? `evm:${chain?.chainId}` : network;
}

function withSettings(network: Network, c: Chain): Chain {
	const key = chainKey(network, c);
	return {
		...c,
		rpc: store.settings.rpc[key] || c.rpc,
		// an endpoint chosen in Settings is the only one used
		fallback: store.settings.rpc[key] ? undefined : c.fallback
	};
}

/** The chains of a network, with the settings applied: those of the mode shown (mainnet or
 * testnet), or with `all`, every one. Bitcoin is mainnet only. */
export function chainsOf(network: Network, all = false): Chain[] {
	const testnet = !!store.settings.testnet;
	if (network === 'evm')
		return [...registry.evm, ...store.settings.chains]
			.filter((c) => all || !!c.testnet === testnet)
			.map((c) => withSettings('evm', c));
	return all || !testnet ? [withSettings(network, registry[network])] : [];
}

const same = (a: Account, b: Account) => a.network === b.network && a.address === b.address;
/** One number of one network with two addresses: two wallets, or one phrase under two
 * passphrases. */
const clash = (a: Account, b: Account) =>
	a.network === b.network && a.index === b.index && a.address !== b.address;

/** The stored accounts these contradict: they are of another wallet. */
export function conflicts(accounts: Account[]): Account[] {
	return store.accounts.filter((b) => accounts.some((a) => clash(a, b)));
}

/** Adds accounts the phone showed; returns how many were new. One that contradicts a stored
 * account is left out: it is of another wallet (see `replaceAccounts`). */
export function addAccounts(accounts: Account[]): number {
	let added = 0;
	for (const a of accounts) {
		if (!store.accounts.some((b) => same(a, b) || clash(a, b))) {
			store.accounts.push(a);
			added++;
		}
	}
	store.accounts.sort((a, b) => a.index - b.index);
	save();
	return added;
}

/** Another wallet's accounts in place of every stored one. */
export function replaceAccounts(accounts: Account[]): number {
	store.accounts = [];
	return addAccounts(accounts);
}

export function removeAccount(a: Account) {
	store.accounts = store.accounts.filter((b) => !same(a, b));
	save();
}
