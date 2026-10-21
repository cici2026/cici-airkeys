// What the web app keeps: the accounts the phone has shown it (addresses only) and the
// settings (endpoints, explorers, extra EVM chains). Both live in this browser's storage and
// nowhere else; nothing here is secret.
import { NETWORKS, registry, type Chain, type Network } from './registry';
import type { Account } from './protocol';

export interface Settings {
	/** Endpoint per chain key: "bitcoin", "evm:<chainId>". */
	rpc: Record<string, string>;
	/** Explorer templates per chain key ("{}" stands for the hash or address). */
	explorer: Record<string, { tx: string; address: string }>;
	/** EVM chains added by hand. */
	chains: Chain[];
	/** Show test networks instead of the real ones: their coins have no value, and the two are
	 * never shown together. */
	testnet?: boolean;
}

const KEY = 'airkeys.v1';

function load(): { accounts: Account[]; settings: Settings } {
	const empty = { accounts: [], settings: { rpc: {}, explorer: {}, chains: [] } };
	try {
		const v = JSON.parse(localStorage.getItem(KEY) ?? 'null');
		if (!v || !Array.isArray(v.accounts)) return empty;
		// accounts of networks Airkeys no longer supports are dropped
		return {
			accounts: v.accounts.filter((a: Account) => NETWORKS.includes(a.network)),
			settings: { ...empty.settings, ...v.settings }
		};
	} catch {
		return empty;
	}
}

export const store = $state(
	typeof localStorage === 'undefined'
		? { accounts: [] as Account[], settings: { rpc: {}, explorer: {}, chains: [] } as Settings }
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
		fallback: store.settings.rpc[key] ? undefined : c.fallback,
		explorer: store.settings.explorer[key] ?? c.explorer
	};
}

/** The chains of a network, with the settings applied. */
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

/** Adds accounts the phone showed; returns how many were new. */
export function addAccounts(accounts: Account[]): number {
	let added = 0;
	for (const a of accounts) {
		if (!store.accounts.some((b) => b.network === a.network && b.address === a.address)) {
			store.accounts.push(a);
			added++;
		}
	}
	store.accounts.sort((a, b) => a.index - b.index);
	save();
	return added;
}

export function removeAccount(a: Account) {
	store.accounts = store.accounts.filter(
		(b) => !(b.network === a.network && b.address === a.address)
	);
	save();
}
