// One interface over the two network families.
import type { Chain, Network } from '../registry';
import { chainsOf } from '../state.svelte';
import * as bitcoin from './bitcoin';
import * as evm from './evm';
import type { Asset, Broadcast, Exposure, Prepared, SendIntent } from './net';

export type { Asset, Broadcast, Exposure, Prepared, SendIntent };

const modules = { bitcoin, evm };

export interface Look {
	assets: Asset[];
	/** Chains that did not answer, with why. */
	failed: string[];
	exposure: Exposure;
	/** It ever received or sent something. */
	used: boolean;
}

/** Bitcoin requests in flight at once: public Esplora servers limit request rates. */
const BITCOIN_PARALLEL = 3;

/** What each address holds on every chain of its network, and whether its key is known. EVM
 * chains are asked in batches across addresses, Bitcoin a few addresses at a time. */
export async function load(network: Network, addresses: string[]): Promise<Map<string, Look>> {
	const out = new Map<string, Look>();
	if (network === 'bitcoin') {
		const chain = chainsOf('bitcoin')[0];
		// Bitcoin is mainnet only: nothing to read on the test networks
		if (!chain) {
			for (const a of addresses)
				out.set(a, { assets: [], failed: [], exposure: 'unknown', used: false });
			return out;
		}
		const queue = [...addresses];
		const worker = async () => {
			for (let a = queue.shift(); a !== undefined; a = queue.shift()) {
				try {
					const r = await bitcoin.look(chain, a);
					out.set(a, {
						assets: r.assets,
						failed: [],
						exposure: r.exposed ? 'exposed' : 'hidden',
						used: r.used
					});
				} catch (e) {
					out.set(a, {
						assets: [],
						failed: [`${chain.name}: ${(e as Error).message}`],
						exposure: 'unknown',
						used: false
					});
				}
			}
		};
		await Promise.all(Array.from({ length: BITCOIN_PARALLEL }, worker));
		return out;
	}
	const chains = chainsOf('evm');
	// the chains of the other mode too, for one thing: a signature on any EVM chain — a test
	// network included — reveals the key for all of them
	const others = chainsOf('evm', true).filter((c) => !chains.some((x) => x.chainId === c.chainId));
	const [scans, signedElsewhere] = await Promise.all([
		Promise.allSettled(chains.map((c) => evm.scan(c, addresses))),
		Promise.allSettled(others.map((c) => evm.nonces(c, addresses)))
	]);
	for (const a of addresses) {
		const look: Look = { assets: [], failed: [], exposure: 'unknown', used: false };
		let signed = false;
		let unsure = false;
		scans.forEach((s, i) => {
			const r = s.status === 'fulfilled' ? s.value.get(a) : undefined;
			if (!r) {
				look.failed.push(
					`${chains[i].name}: ${s.status === 'rejected' ? (s.reason as Error).message : 'no answer'}`
				);
				unsure = true;
				return;
			}
			look.assets.push(...r.assets);
			if (r.nonce === null) unsure = true;
			else if (r.nonce > 0n) signed = true;
		});
		for (const s of signedElsewhere) {
			const n = s.status === 'fulfilled' ? s.value.get(a) : undefined;
			if (n === undefined || n === null) unsure = true;
			else if (n > 0n) signed = true;
		}
		look.exposure = signed ? 'exposed' : unsure ? 'unknown' : 'hidden';
		look.used = signed || look.assets.some((x) => x.amount > 0n);
		out.set(a, look);
	}
	return out;
}

export function prepare(network: Network, intent: SendIntent): Promise<Prepared> {
	return modules[network].prepare(intent);
}

export function broadcast(network: Network, chain: Chain, raws: string[]): Promise<Broadcast[]> {
	return modules[network].broadcast(chain, raws);
}
