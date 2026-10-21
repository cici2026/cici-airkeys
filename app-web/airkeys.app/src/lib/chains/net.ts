// HTTP for the chain clients: JSON-RPC and JSON over HTTP, with a timeout and errors that say
// which endpoint failed.
import type { Chain, Token } from '../registry';
import type { Account, SignRequest, Target } from '../protocol';

const TIMEOUT_MS = 20_000;

async function once(url: string, init: RequestInit): Promise<Response> {
	const ctl = new AbortController();
	const t = setTimeout(() => ctl.abort(), TIMEOUT_MS);
	try {
		return await fetch(url, { ...init, signal: ctl.signal });
	} catch (e) {
		throw new Error(
			`${new URL(url).host} did not answer${(e as Error).name === 'AbortError' ? ' in time' : ''}`
		);
	} finally {
		clearTimeout(t);
	}
}

/** A request, asked again after 1, 3 and 9 s while the server says it is busy (HTTP 429):
 * public endpoints limit how fast one visitor may ask. */
async function request(url: string, init: RequestInit): Promise<Response> {
	for (let attempt = 0; ; attempt++) {
		const r = await once(url, init);
		if (r.status !== 429 || attempt === 3) return r;
		await new Promise((ok) => setTimeout(ok, 1000 * 3 ** attempt));
	}
}

export async function getJson<T = any>(url: string): Promise<T> {
	const r = await request(url, { headers: { accept: 'application/json' } });
	if (!r.ok) throw new Error(`${new URL(url).host}: HTTP ${r.status}`);
	return r.json();
}

export async function postJson<T = any>(url: string, body: unknown): Promise<T> {
	const r = await request(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!r.ok) throw new Error(`${new URL(url).host}: HTTP ${r.status}`);
	return r.json();
}

export async function getText(url: string): Promise<string> {
	const r = await request(url, {});
	if (!r.ok) throw new Error(`${new URL(url).host}: HTTP ${r.status}`);
	return r.text();
}

export async function postText(
	url: string,
	body: string
): Promise<{ ok: boolean; status: number; text: string }> {
	const r = await request(url, { method: 'POST', headers: { 'content-type': 'text/plain' }, body });
	return { ok: r.ok, status: r.status, text: await r.text() };
}

let rpcId = 0;
export async function rpc<T = any>(
	url: string,
	method: string,
	params: unknown[] = []
): Promise<T> {
	const j = await postJson(url, { jsonrpc: '2.0', id: ++rpcId, method, params });
	if (j.error) throw new Error(`${method}: ${j.error.message ?? JSON.stringify(j.error)}`);
	return j.result;
}

/** Several calls in one JSON-RPC batch; a failed call gives `null`. */
export async function rpcBatch(url: string, calls: [string, unknown[]][]): Promise<any[]> {
	if (!calls.length) return [];
	const base = ++rpcId * 1000;
	const out = await postJson<any[]>(
		url,
		calls.map(([method, params], i) => ({ jsonrpc: '2.0', id: base + i, method, params }))
	);
	const byId = new Map(out.map((r) => [r.id, r]));
	return calls.map((_, i) => byId.get(base + i)?.result ?? null);
}

/** One coin or token an account holds on one chain. */
export interface Asset {
	key: string;
	chain: Chain;
	/** Absent for the native coin. */
	token?: Token;
	symbol: string;
	decimals: number;
	amount: bigint;
}

export type Exposure = 'hidden' | 'exposed' | 'unknown';

/** What a send asks for. */
export interface SendIntent {
	from: Account;
	asset: Asset;
	/** The recipient's address. */
	to: string;
	/** Base units; `null` sends everything. */
	amount: bigint | null;
	/** Move what is left of every asset on this chain to this account of the wallet. */
	moveRest: number | null;
	/** Everything the account holds on the chain (for `moveRest`). */
	holdings: Asset[];
	/** Fee level, 0 (slow) to 2 (fast). */
	speed: 0 | 1 | 2;
}

export interface Prepared {
	request: SignRequest;
	/** What the request does, line by line, before the phone shows it again. */
	summary: string[];
	/** The most it can cost in fees, as text. */
	fee: string;
}

export interface Broadcast {
	hash: string;
	ok: boolean;
	error?: string;
}

export const own = (index: number): Target => ({ index });
