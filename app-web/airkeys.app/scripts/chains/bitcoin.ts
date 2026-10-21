// The web app's Bitcoin client against Bitcoin Core: it reads balances, builds requests, the
// phone signs them, the web app broadcasts them, and regtest mines them.
//
//   BITCOIND=/path/to/bitcoind bun scripts/chains/bitcoin.ts
//
// Bitcoin Core speaks JSON-RPC and the web app speaks Esplora (mempool.space), so a small
// Esplora built on the node's RPC sits between them. Accounts are mainnet addresses; the
// shim reads them as their regtest twins (the same script).
import { createServer } from 'node:http';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { base58check as b58c, bech32, bech32m } from '@scure/base';
import { sha256 } from '@noble/hashes/sha2.js';
import * as btc from '../../src/lib/chains/bitcoin';
import { BTC_OUTPUT_BYTES } from '../../src/lib/addresses';
import { registry } from '../../src/lib/registry';
import type { SendIntent } from '../../src/lib/chains/net';
import { accounts, check, done, refuses, sign, start } from './lib';

const BITCOIND = process.env.BITCOIND ?? 'bitcoind';
const RPC = 18610;
const ESPLORA = 18611;
const FEES = { hourFee: 2, halfHourFee: 3, fastestFee: 5 };
const base58check = b58c(sha256);

// ------------------------------------------------------------ one script, two spellings
const segwit = (a: string) => (a.startsWith('bc1p') || a.startsWith('bcrt1p') ? bech32m : bech32);
function respell(a: string, to: 'main' | 'regtest'): string {
	if (/^(bc|bcrt)1/i.test(a)) {
		const { words } = segwit(a).decode(a as `${string}1${string}`);
		return segwit(a).encode(to === 'main' ? 'bc' : 'bcrt', words);
	}
	const p = base58check.decode(a);
	const legacy = { main: [0x00, 0x05], regtest: [0x6f, 0xc4] };
	const kind = p[0] === legacy.main[0] || p[0] === legacy.regtest[0] ? 0 : 1;
	return base58check.encode(Uint8Array.from([legacy[to][kind], ...p.subarray(1)]));
}

// ------------------------------------------------------------ the node
const dir = mkdtempSync(path.join(tmpdir(), 'airkeys-web-regtest-'));
process.on('exit', () => rmSync(dir, { recursive: true, force: true }));
await start(
	BITCOIND,
	[
		'-regtest',
		`-datadir=${dir}`,
		`-rpcport=${RPC}`,
		'-rpcuser=a',
		'-rpcpassword=b',
		'-txindex=1',
		'-fallbackfee=0.0002',
		'-connect=0',
		'-printtoconsole'
	],
	'init message: Done loading'
);
async function node(method: string, params: unknown[] = [], wallet = '') {
	const r = await fetch(`http://127.0.0.1:${RPC}/${wallet && `wallet/${wallet}`}`, {
		method: 'POST',
		headers: { authorization: 'Basic ' + btoa('a:b') },
		body: JSON.stringify({ jsonrpc: '1.0', id: 1, method, params })
	});
	const j = await r.json();
	if (j.error) throw new Error(`${method}: ${j.error.message}`);
	return j.result;
}
const sats = (btc: number) => BigInt(Math.round(btc * 1e8));

// ------------------------------------------------------------ Esplora, the parts the web app uses
type Out = { txid: string; vout: number; value: number };
async function history(address: string) {
	const funded: (Out & { confirmed: boolean })[] = [];
	const spent = new Set<string>();
	const scan = (tx: any, confirmed: boolean) => {
		for (const o of tx.vout)
			if (o.scriptPubKey.address === address)
				funded.push({ txid: tx.txid, vout: o.n, value: Number(sats(o.value)), confirmed });
		for (const i of tx.vin)
			if (i.prevout?.scriptPubKey.address === address) spent.add(`${i.txid}:${i.vout}`);
	};
	const height = await node('getblockcount');
	for (let h = 0; h <= height; h++)
		for (const tx of (await node('getblock', [await node('getblockhash', [h]), 3])).tx)
			scan(tx, true);
	for (const txid of await node('getrawmempool'))
		scan(await node('getrawtransaction', [txid, 2]), false);
	return { funded, spent };
}
const stats = (outs: (Out & { confirmed: boolean })[], spent: Set<string>, confirmed: boolean) => {
	const mine = outs.filter((o) => o.confirmed === confirmed);
	const gone = mine.filter((o) => spent.has(`${o.txid}:${o.vout}`));
	return {
		funded_txo_sum: mine.reduce((s, o) => s + o.value, 0),
		spent_txo_sum: gone.reduce((s, o) => s + o.value, 0),
		spent_txo_count: gone.length
	};
};

const esplora = createServer(async (req, res) => {
	const send = (code: number, body: unknown) => {
		res.writeHead(code, {
			'content-type': typeof body === 'string' ? 'text/plain' : 'application/json',
			'access-control-allow-origin': '*'
		});
		res.end(typeof body === 'string' ? body : JSON.stringify(body));
	};
	try {
		const url = new URL(req.url!, 'http://x');
		let m: RegExpMatchArray | null;
		if (req.method === 'POST' && url.pathname === '/tx') {
			let body = '';
			for await (const c of req) body += c;
			try {
				send(200, await node('sendrawtransaction', [body.trim()]));
			} catch (e) {
				send(400, (e as Error).message);
			}
		} else if (url.pathname === '/v1/fees/recommended') send(200, FEES);
		else if ((m = url.pathname.match(/^\/tx\/(\w+)\/hex$/)))
			send(200, await node('getrawtransaction', [m[1]]));
		else if ((m = url.pathname.match(/^\/address\/(\w+)(\/utxo)?$/))) {
			const { funded, spent } = await history(respell(m[1], 'regtest'));
			if (m[2])
				send(
					200,
					funded
						.filter((o) => !spent.has(`${o.txid}:${o.vout}`))
						.map(({ txid, vout, value, confirmed }) => ({
							txid,
							vout,
							value,
							status: { confirmed }
						}))
				);
			else
				send(200, {
					chain_stats: stats(funded, spent, true),
					mempool_stats: stats(funded, spent, false)
				});
		} else send(404, 'not found');
	} catch (e) {
		send(500, (e as Error).message);
	}
});
await new Promise<void>((ok) => esplora.listen(ESPLORA, '127.0.0.1', ok));
const chain = { ...registry.bitcoin, rpc: `http://127.0.0.1:${ESPLORA}`, fallback: undefined };

// ------------------------------------------------------------ the checks
await node('createwallet', ['miner']);
const fresh = async (type: string) =>
	respell(await node('getnewaddress', ['', type], 'miner'), 'main');
const mine = async (n = 1) =>
	node('generatetoaddress', [n, respell(await fresh('bech32'), 'regtest')]);
await mine(101);
const received = async (address: string) =>
	sats(await node('getreceivedbyaddress', [respell(address, 'regtest'), 0], 'miner'));

const [a0, a1, a2] = accounts('bitcoin', 0, 1, 2);
for (const amount of [0.5, 0.3, 0.2])
	await node('sendtoaddress', [respell(a0.address, 'regtest'), amount], 'miner');
await mine();

const assets = async (address: string) => btc.balances(chain, address);
const intent = async (
	from: typeof a0,
	to: string,
	amount: bigint | null,
	moveRest: number | null
): Promise<SendIntent> => {
	const holdings = await assets(from.address);
	return { from, asset: holdings[0], to, amount, moveRest, holdings, speed: 1 };
};

check(
	(await assets(a0.address))[0].amount === 100_000_000n,
	'the web app reads account #0: 1 BTC in three coins'
);
check(!(await btc.exposed(chain, a0.address)), 'account #0 has never spent: its key is hidden');

// 1. pay 0.4 BTC to a taproot address; the change moves to account #1
const taproot = await fresh('bech32m');
const p1 = await btc.prepare(await intent(a0, taproot, 40_000_000n, 1));
const rate = BigInt(FEES.halfHourFee);
const fee1 = rate * BigInt(btc.vsize(3, [BTC_OUTPUT_BYTES.p2tr, BTC_OUTPUT_BYTES.p2wpkh]));
check(
	(p1.request.inputs as unknown[]).length === 3 && (p1.request.outputs as unknown[]).length === 2,
	'the request spends all three coins, to the recipient and the change'
);
const s1 = sign(p1.request);
check(
	s1.accounts.some((a) => a.index === 1 && a.address === a1.address),
	'the reply names account #1, as the phone derived it'
);
const b1 = await btc.broadcast(
	chain,
	s1.txs.map((t) => t.raw)
);
check(
	b1.every((b) => b.ok) && b1[0].hash === s1.txs[0].hash,
	`the web app broadcast it: ${b1[0].hash.slice(0, 16)}…`
);
const tx1 = await node('getmempoolentry', [b1[0].hash]);
check(
	sats(tx1.fees.base) === fee1 && tx1.vsize <= btc.vsize(3, [43, 31]),
	`the fee is the one shown, ${fee1} sat (${rate} sat/vB on the estimate; the real size is ${tx1.vsize} vB)`
);
await mine();
check((await received(taproot)) === 40_000_000n, 'the taproot recipient received 0.4 BTC');
check(
	(await assets(a1.address))[0].amount === 100_000_000n - 40_000_000n - fee1,
	'account #1 holds the change'
);
check(
	(await assets(a0.address))[0].amount === 0n && (await btc.exposed(chain, a0.address)),
	'account #0 is empty, and the web app now marks its key exposed'
);

// 2. send everything from account #1 to a legacy address
const legacy = await fresh('legacy');
const all = (await assets(a1.address))[0].amount;
const p2 = await btc.prepare(await intent(a1, legacy, null, 2));
const fee2 = rate * BigInt(btc.vsize(1, [BTC_OUTPUT_BYTES.p2pkh]));
const b2 = await btc.broadcast(
	chain,
	sign(p2.request).txs.map((t) => t.raw)
);
await mine();
check(
	b2[0].ok && (await received(legacy)) === all - fee2,
	`"send all" paid ${all - fee2} sat to a legacy address; the fee was ${fee2} sat`
);
check(
	(await assets(a1.address))[0].amount === 0n,
	'account #1 is empty: no change, nothing left behind'
);

// 3. what the web app will not build
await node('sendtoaddress', [respell(a2.address, 'regtest'), 0.001], 'miner');
await mine();
await refuses('sending back to the paying account', async () =>
	btc.prepare(await intent(a2, a2.address, 10_000n, 3))
);
await refuses('more than the account holds', async () =>
	btc.prepare(await intent(a2, taproot, 100_001n, 3))
);
await refuses('an amount that leaves too little for its fee', async () =>
	btc.prepare(await intent(a2, taproot, 99_990n, 3))
);
await refuses('below the dust limit', async () => btc.prepare(await intent(a2, taproot, 300n, 3)));
await refuses('an account with no coins', async () =>
	btc.prepare(await intent(a0, taproot, 10_000n, 3))
);
// the phone has the last word: a request the web app would never build, with a forged amount
const forged = await btc.prepare(await intent(a2, taproot, 50_000n, 3));
(forged.request.inputs as { amount: string }[])[0].amount = '200000';
await refuses('the phone, given an input amount its previous transaction contradicts', () =>
	sign(forged.request)
);

esplora.close();
done();
