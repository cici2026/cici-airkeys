// The Airkeys wire protocol, version 1 (docs/PROTOCOL.md), from the web app's side: it reads
// `accounts` and `signed`, and writes `sign`. Reading is as strict as the phone: unknown
// fields, wrong types or another version are refused.
import { NETWORKS, type Network } from './registry';
import { parseAddress } from './addresses';

export const VERSION = 1;
/** Every account number the phone offers (100) on both networks (PROTOCOL §4). */
export const MAX_ACCOUNTS = 200;
/** The largest message, in bytes (PROTOCOL §2.6). */
export const MAX_MESSAGE = 1 << 20;

export interface Account {
	network: Network;
	index: number;
	address: string;
}

/** An address, an account of the wallet, or both (PROTOCOL §3). */
export interface Target {
	index?: number;
	address?: string;
}

export interface SignedTx {
	hash: string;
	raw: string;
}

export interface Signed {
	id: string;
	network: Network;
	txs: SignedTx[];
	accounts: Account[];
}

export type Incoming =
	{ type: 'accounts'; accounts: Account[] } | { type: 'signed'; signed: Signed };

/** A sign request: the envelope, then the network's fields (§5). */
export type SignRequest = {
	airkeys: 1;
	type: 'sign';
	id: string;
	network: Network;
	from: { index: number; address: string };
} & Record<string, unknown>;

class Refused extends Error {}

const isObject = (v: unknown): v is Record<string, unknown> =>
	typeof v === 'object' && v !== null && !Array.isArray(v);
/** An account number: 0 to 2³¹−1 (PROTOCOL §3). */
export const isIndex = (v: unknown): v is number =>
	Number.isInteger(v) && (v as number) >= 0 && (v as number) <= 0x7fffffff;

function only(o: Record<string, unknown>, keys: string[], what: string) {
	const extra = Object.keys(o).filter((k) => !keys.includes(k));
	if (extra.length) throw new Refused(`The ${what} has an unknown field “${extra[0]}”.`);
}

function account(v: unknown): Account {
	if (!isObject(v)) throw new Refused('An account is not an object.');
	only(v, ['network', 'index', 'address'], 'account');
	if (!NETWORKS.includes(v.network as Network))
		throw new Refused(`“${String(v.network)}” is not a network.`);
	if (!isIndex(v.index)) throw new Refused('An account number is not valid.');
	if (typeof v.address !== 'string' || !v.address) throw new Refused('An account has no address.');
	// as strict as the phone: an address of its network, spelled the way the phone spells it
	let canonical = '';
	try {
		canonical = parseAddress(v.network as Network, v.address);
	} catch {
		/* refused below */
	}
	if (canonical !== v.address) throw new Refused('An account address is not valid.');
	return { network: v.network as Network, index: v.index, address: v.address };
}

/** Accounts, each network and number at most once. */
function accountList(list: unknown[]): Account[] {
	const accounts = list.map(account);
	const seen = new Set(accounts.map((a) => `${a.network}:${a.index}`));
	if (seen.size !== accounts.length) throw new Refused('An account is listed twice.');
	return accounts;
}

const HASH: Record<Network, RegExp> = { bitcoin: /^[0-9a-f]{64}$/, evm: /^0x[0-9a-f]{64}$/ };
const RAW: Record<Network, RegExp> = { bitcoin: /^(?:[0-9a-f]{2})+$/, evm: /^0x(?:[0-9a-f]{2})+$/ };

/** Parses a message the phone sent (§4, §6). */
export function parseMessage(bytes: Uint8Array): Incoming {
	if (bytes.length > MAX_MESSAGE) throw new Refused('The message is larger than 1 MiB.');
	let v: unknown;
	try {
		v = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
	} catch {
		throw new Refused('This is not an Airkeys message.');
	}
	if (!isObject(v) || typeof v.airkeys !== 'number')
		throw new Refused('This is not an Airkeys message.');
	if (v.airkeys !== VERSION)
		throw new Refused(
			`This message uses Airkeys protocol version ${v.airkeys}. This app speaks version ${VERSION}.`
		);
	switch (v.type) {
		case 'accounts': {
			only(v, ['airkeys', 'type', 'accounts'], 'message');
			if (!Array.isArray(v.accounts) || v.accounts.length < 1 || v.accounts.length > MAX_ACCOUNTS)
				throw new Refused(`An accounts message lists 1 to ${MAX_ACCOUNTS} accounts.`);
			return { type: 'accounts', accounts: accountList(v.accounts) };
		}
		case 'signed': {
			only(v, ['airkeys', 'type', 'id', 'network', 'txs', 'accounts'], 'message');
			if (
				typeof v.id !== 'string' ||
				!/^[A-Za-z0-9_-]{1,64}$/.test(v.id) ||
				!NETWORKS.includes(v.network as Network) ||
				!Array.isArray(v.txs) ||
				v.txs.length < 1 ||
				v.txs.length > 16 ||
				(v.accounts !== undefined && !Array.isArray(v.accounts))
			)
				throw new Refused('The signed message is not valid.');
			const network = v.network as Network;
			const txs = v.txs.map((t) => {
				if (!isObject(t) || typeof t.hash !== 'string' || typeof t.raw !== 'string')
					throw new Refused('A signed transaction is not valid.');
				only(t, ['hash', 'raw'], 'signed transaction');
				if (!HASH[network].test(t.hash) || !RAW[network].test(t.raw))
					throw new Refused('A signed transaction is not valid.');
				return { hash: t.hash, raw: t.raw };
			});
			const accounts = accountList((v.accounts as unknown[] | undefined) ?? []);
			if (accounts.some((a) => a.network !== network))
				throw new Refused('The signed message names an account of another network.');
			return { type: 'signed', signed: { id: v.id, network, txs, accounts } };
		}
		case 'sign':
			throw new Refused('This is a request from a web app, not a reply from the phone.');
		default:
			throw new Refused(`“${String(v.type)}” is not an Airkeys message type.`);
	}
}

/** A fresh request id: 16 hex digits. */
export function newId(): string {
	return Array.from(crypto.getRandomValues(new Uint8Array(8)), (b) =>
		b.toString(16).padStart(2, '0')
	).join('');
}

export function signRequest(
	network: Network,
	from: Account,
	fields: Record<string, unknown>
): SignRequest {
	return {
		airkeys: 1,
		type: 'sign',
		id: newId(),
		network,
		...(fields.chainId !== undefined ? { chainId: fields.chainId } : {}),
		from: { index: from.index, address: from.address },
		...withoutChainId(fields)
	} as SignRequest;
}

function withoutChainId(f: Record<string, unknown>) {
	const { chainId: _, ...rest } = f;
	return rest;
}

export function encode(message: object): Uint8Array {
	return new TextEncoder().encode(JSON.stringify(message));
}
