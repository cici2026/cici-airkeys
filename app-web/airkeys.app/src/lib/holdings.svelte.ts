// Balances and key exposure per address, fetched on demand and kept for the session.
import { load, type Look } from './chains';
import type { Account } from './protocol';
import type { Network } from './registry';
import { store } from './state.svelte';
import { t } from './i18n/index.svelte';

export interface Holding extends Look {
	loading: boolean;
	/** Not asked: past the gap of unused Bitcoin accounts (see `ensure`). */
	skipped?: boolean;
}

const cache = $state<Record<string, Holding>>({});
// per mode: what a test network holds never mixes with what the real ones hold
const key = (a: Account) => `${store.settings.testnet ? 'test' : 'main'}:${a.network}:${a.address}`;
const blank = { assets: [], failed: [], exposure: 'unknown' as const, used: false };
/** EVM accounts per round of requests: results show up as each round finishes. */
const ROUND = 25;
/** Bitcoin asks one address per request, so it stops after this many unused accounts in a
 * row — as Bitcoin wallets do. Bunker mode moves funds to the lowest unused account, so used
 * accounts sit at the low numbers. */
const GAP = 10;

export function holdingOf(a: Account): Holding | undefined {
	return cache[key(a)];
}

async function round(network: Network, accounts: Account[]) {
	let looks = new Map<string, Look>();
	let failed = t.common.noAnswerAtAll;
	try {
		looks = await load(
			network,
			accounts.map((a) => a.address)
		);
	} catch (e) {
		failed = (e as Error).message;
	}
	for (const a of accounts) {
		cache[key(a)] = {
			...(looks.get(a.address) ?? { ...blank, failed: [failed] }),
			loading: false
		};
	}
}

/** Loads these accounts' holdings (again); it never fails: an account that could not be read
 * says why. With `gap`, Bitcoin accounts are read in order and the reading stops after `GAP`
 * unused ones in a row; the rest are marked skipped. */
export async function refresh(accounts: Account[], gap = false) {
	for (const a of accounts)
		cache[key(a)] = { ...(cache[key(a)] ?? blank), loading: true, skipped: false };
	const evm = accounts.filter((a) => a.network === 'evm');
	const btc = accounts.filter((a) => a.network === 'bitcoin').sort((a, b) => a.index - b.index);
	await Promise.all([
		(async () => {
			for (let i = 0; i < evm.length; i += ROUND) await round('evm', evm.slice(i, i + ROUND));
		})(),
		(async () => {
			let unused = 0;
			for (let i = 0; i < btc.length; i += 3) {
				if (gap && unused >= GAP) {
					for (const a of btc.slice(i)) cache[key(a)] = { ...blank, loading: false, skipped: true };
					return;
				}
				const some = btc.slice(i, i + 3);
				await round('bitcoin', some);
				for (const a of some) {
					// (gone when the mode switched meanwhile)
					const h = cache[key(a)];
					unused = !h || h.used || h.failed.length ? 0 : unused + 1;
				}
			}
		})()
	]);
}

/** Loads the accounts not loaded yet, Bitcoin up to the gap. */
export function ensure(...accounts: Account[]) {
	const missing = accounts.filter((a) => !cache[key(a)]);
	if (missing.length) void refresh(missing, true);
}

/** The account the rest moves to: the lowest connected account, other than `except`, that the
 * web app has read and seen never used. `null` while none is known to be unused — then more
 * accounts must be connected (or read) first; an account it has not seen is not assumed new.
 * A number stored with two addresses (two wallets connected here) is never chosen: in one of
 * them it may be used. */
export function freshIndex(network: Network, except: number): number | null {
	const mine = store.accounts.filter((a) => a.network === network);
	const doubtful = new Set(
		mine
			.filter((a) => mine.some((b) => b.index === a.index && b.address !== a.address))
			.map((a) => a.index)
	);
	const fresh = mine
		.filter((a) => a.index !== except && !doubtful.has(a.index))
		.filter((a) => {
			const h = holdingOf(a);
			return (
				!!h && !h.loading && !h.skipped && !h.failed.length && !h.used && h.exposure === 'hidden'
			);
		})
		.map((a) => a.index);
	return fresh.length ? Math.min(...fresh) : null;
}
