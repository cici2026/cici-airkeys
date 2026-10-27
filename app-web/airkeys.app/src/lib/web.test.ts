// The web app's own code against the phone's core: the same addresses accepted and refused,
// the same transaction sizes (vectors.json comes from
// core/wallet/examples/vectors.rs; regenerate it when the core changes).
import { afterEach, describe, expect, it, vi } from 'vitest';
import { hex } from '@scure/base';
import vectors from './vectors.json';
import { parseBitcoin, parseEvm, paysSameKey } from './addresses';
import * as bitcoin from './chains/bitcoin';
import * as evm from './chains/evm';
import type { Asset, SendIntent } from './chains/net';
import { registry } from './registry';
import { formatShort, formatUnits, parseUnits } from './units';
import { parseMessage, encode } from './protocol';
import { elsewhere, priceKey, readRound, summarize, STALE_SECONDS, type Holder } from './value';

describe('addresses agree with the core', () => {
	it('Bitcoin', () => {
		for (const v of vectors.bitcoin) {
			if (v.script === null) expect(() => parseBitcoin(v.address), v.address).toThrow();
			else expect(hex.encode(parseBitcoin(v.address).script)).toBe(v.script);
		}
	});
	it('EVM', () => {
		for (const v of vectors.evm) {
			if (v.checksum === null) expect(() => parseEvm(v.address)).toThrow();
			else expect(parseEvm(v.address)).toBe(v.checksum);
		}
	});
});

describe('amounts', () => {
	it('parses and formats exactly', () => {
		expect(parseUnits('1.5', 18)).toBe(1_500_000_000_000_000_000n);
		expect(parseUnits('0.000001', 6)).toBe(1n);
		expect(parseUnits('1,000', 6)).toBe(1_000_000_000n);
		expect(parseUnits('1,234.5', 2)).toBe(123_450n);
		// a decimal comma is refused, never read as a thousands separator
		expect(parseUnits('0,5', 8)).toBeNull();
		expect(parseUnits('12,34', 2)).toBeNull();
		expect(parseUnits('0.0000001', 6)).toBeNull();
		expect(parseUnits('1e5', 6)).toBeNull();
		expect(parseUnits('', 6)).toBeNull();
		expect(formatUnits(123456789n, 6)).toBe('123.456789');
		expect(formatUnits(2_100_000_000_000_000n, 8)).toBe('21,000,000');
		expect(formatShort(1n, 18)).toBe('0.000000000000000001');
		expect(formatShort(123_456_789n, 18)).toBe('0.000000000123456');
		expect(formatShort(1_234_567n, 6)).toBe('1.2345');
	});
});

describe('Bitcoin sizes', () => {
	it('match what Bitcoin Core measured for the regtest spend', () => {
		// core/e2e/bitcoin.mjs: 3 P2WPKH inputs; P2TR, P2PKH, P2SH and P2WPKH outputs → 355 vB
		expect(bitcoin.vsize(3, [43, 34, 32, 31])).toBe(355);
	});
});

describe('messages from the phone', () => {
	const accounts = {
		airkeys: 1,
		type: 'accounts',
		accounts: [{ network: 'evm', index: 0, address: '0xF23521DE6e2B111B384c1308b3c59Acc7Bf5e158' }]
	};
	it('reads accounts and signed replies', () => {
		const m = parseMessage(encode(accounts));
		expect(m.type).toBe('accounts');
		const signed = parseMessage(
			encode({
				airkeys: 1,
				type: 'signed',
				id: 'a',
				network: 'evm',
				txs: [{ hash: '0x' + 'ab'.repeat(32), raw: '0x02f86c' }]
			})
		);
		expect(signed.type === 'signed' && signed.signed.accounts).toEqual([]);
		const reply = (t: object) =>
			encode({ airkeys: 1, type: 'signed', id: 'a', network: 'evm', ...t });
		const tx = { hash: '0x' + 'ab'.repeat(32), raw: '0x02f86c' };
		// as strict as the phone: hashes, raw transactions, accounts
		expect(() => parseMessage(reply({ txs: [{ hash: '0x1', raw: '0x02' }] }))).toThrow(/not valid/);
		expect(() => parseMessage(reply({ txs: [tx], accounts: {} }))).toThrow(/not valid/);
		expect(() =>
			parseMessage(
				reply({ txs: [tx], accounts: [{ network: 'evm', index: 1, address: 'nonsense' }] })
			)
		).toThrow(/address is not valid/);
		expect(() =>
			parseMessage(
				reply({
					txs: [tx],
					accounts: [
						{ network: 'bitcoin', index: 1, address: 'bc1qvuapkt7kdu4vnrfttvzmszuj8cfdnedsch0hy3' }
					]
				})
			)
		).toThrow(/another network/);
	});
	it('refuses what the phone would refuse', () => {
		expect(() => parseMessage(encode({ ...accounts, airkeys: 2 }))).toThrow(/version 2/);
		expect(() => parseMessage(encode({ ...accounts, extra: 1 }))).toThrow(/unknown field/);
		expect(() =>
			parseMessage(encode({ ...accounts, accounts: [{ ...accounts.accounts[0], path: "m/0'" }] }))
		).toThrow(/unknown field/);
		expect(() => parseMessage(encode({ ...accounts, accounts: [] }))).toThrow(/1 to 200/);
		expect(() =>
			parseMessage(encode({ ...accounts, accounts: [accounts.accounts[0], accounts.accounts[0]] }))
		).toThrow(/twice/);
		expect(() => parseMessage(new TextEncoder().encode('not json'))).toThrow(/not an Airkeys/);
	});
});

// ------------------------------------------------------------------ building requests

/** The test phrase's account #0 (PROTOCOL §4). */
const BTC_FROM = {
	network: 'bitcoin' as const,
	index: 0,
	address: 'bc1qvuapkt7kdu4vnrfttvzmszuj8cfdnedsch0hy3'
};
const EVM_FROM = {
	network: 'evm' as const,
	index: 0,
	address: '0xF23521DE6e2B111B384c1308b3c59Acc7Bf5e158'
};
const STRANGER_BTC = 'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4';
const STRANGER_EVM = '0x8617E340B3D01FA5F11F306F4090FD50E238070D';

const eth = registry.evm[0];
const [USDC, , DAI] = eth.tokens;
const coin = (amount: bigint): Asset => ({
	key: `${eth.chainId}:native`,
	chain: eth,
	symbol: eth.symbol,
	decimals: eth.decimals,
	amount
});
const token = (t: typeof USDC, amount: bigint): Asset => ({
	key: `${eth.chainId}:${t.address}`,
	chain: eth,
	token: t,
	symbol: t.symbol,
	decimals: t.decimals,
	amount
});
const sats = (amount: bigint): Asset => ({
	key: 'btc',
	chain: registry.bitcoin,
	symbol: 'BTC',
	decimals: 8,
	amount
});
const intent = (
	i: Partial<SendIntent> & Pick<SendIntent, 'from' | 'asset' | 'to'>
): SendIntent => ({
	amount: null,
	moveRest: null,
	holdings: [i.asset],
	speed: 1,
	...i
});

const answer = (body: unknown) =>
	new Response(typeof body === 'string' ? body : JSON.stringify(body), { status: 200 });

/** A chain node: 10 gwei base fee, 1 gwei tip, nonce 7, no layer-1 fee; 21,000 gas for a coin,
 * 60,000 estimated for a token transfer. */
const GWEI = 1_000_000_000n;
const MAX_FEE = 2n * 10n * GWEI + GWEI;
function evmNode() {
	vi.stubGlobal('fetch', async (_url: string, init: RequestInit) => {
		const { id, method, params } = JSON.parse(init.body as string);
		const result = {
			eth_getBlockByNumber: { baseFeePerGas: '0x' + (10n * GWEI).toString(16) },
			eth_maxPriorityFeePerGas: '0x' + GWEI.toString(16),
			eth_getTransactionCount: '0x7',
			eth_getCode: '0x',
			eth_estimateGas: params[0]?.data ? '0xea60' : '0x5208'
		}[method as string];
		return answer({ jsonrpc: '2.0', id, result });
	});
}

/** An Esplora API holding these coins of the sending account, at 10 sat/vB (normal). */
function esploraNode(values: number[]) {
	vi.stubGlobal('fetch', async (url: string) => {
		const path = new URL(url).pathname;
		if (path.endsWith('/utxo'))
			return answer(values.map((value, vout) => ({ txid: 'ab'.repeat(32), vout, value })));
		if (path.endsWith('/fees/recommended'))
			return answer({ hourFee: 5, halfHourFee: 10, fastestFee: 20 });
		if (path.endsWith('/hex')) return answer('02000000' + '00'.repeat(60));
		return new Response('', { status: 404 });
	});
}

type Tx = { to: { address?: string; index?: number }; amount: string; gas: number; nonce: number };

describe('transfer requests', () => {
	afterEach(() => vi.unstubAllGlobals());

	it('refuse, as the phone does, a transfer to the sending key', async () => {
		// the key of private key 1 in its three forms (BIP-173's example, and its well-known
		// legacy and nested addresses)
		const own = 'bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4';
		for (const to of [
			own,
			own.toUpperCase(),
			'1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH',
			'3JvL6Ymt8MVWiCNHC7oWU6nLeHNJKLZGLN'
		])
			expect(paysSameKey(own, to), to).toBe(true);
		expect(paysSameKey(own, BTC_FROM.address)).toBe(false);
		expect(paysSameKey(own, '1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2')).toBe(false);

		const fetch = vi.fn();
		vi.stubGlobal('fetch', fetch);
		for (const to of [
			BTC_FROM.address,
			BTC_FROM.address.toUpperCase(),
			'1AQpD3nTnF5QfveyjU3i3q8VM5fxvnFA16',
			'3EWQsUR11qUMBz12boarGEbRckawjc9ayL'
		])
			await expect(
				bitcoin.prepare(intent({ from: BTC_FROM, asset: sats(100_000n), to })),
				to
			).rejects.toThrow(/another address/);
		for (const to of [EVM_FROM.address, EVM_FROM.address.toLowerCase()])
			await expect(
				evm.prepare(intent({ from: EVM_FROM, asset: coin(GWEI), to })),
				to
			).rejects.toThrow(/another address/);
		// refused before asking anything
		expect(fetch).not.toHaveBeenCalled();
	});

	it('EVM: sending all of the coin leaves its fee', async () => {
		evmNode();
		const balance = 10n ** 18n;
		const { request } = await evm.prepare(
			intent({ from: EVM_FROM, asset: coin(balance), to: STRANGER_EVM })
		);
		const txs = request.txs as Tx[];
		expect(request.chainId).toBe(1);
		expect(request.from).toEqual({ index: 0, address: EVM_FROM.address });
		expect(txs).toHaveLength(1);
		expect(txs[0]).toMatchObject({
			to: { address: STRANGER_EVM },
			nonce: 7,
			gas: 21000,
			maxFeePerGas: MAX_FEE.toString(),
			maxPriorityFeePerGas: GWEI.toString()
		});
		expect(BigInt(txs[0].amount)).toBe(balance - 21000n * MAX_FEE);
	});

	it('EVM: the rest moves to a fresh account, the fees kept back', async () => {
		evmNode();
		const balance = 10n ** 18n;
		const holdings = [coin(balance), token(USDC, 1000_000000n), token(DAI, 5n * 10n ** 18n)];
		const { request, summary } = await evm.prepare(
			intent({
				from: EVM_FROM,
				asset: holdings[1],
				to: STRANGER_EVM,
				amount: 100_000000n,
				moveRest: 1,
				holdings
			})
		);
		const txs = request.txs as Tx[];
		expect(txs.map((t) => [t.to, t.amount, t.nonce])).toEqual([
			[{ address: STRANGER_EVM }, '100000000', 7],
			[{ index: 1 }, '900000000', 8],
			[{ index: 1 }, '5000000000000000000', 9],
			[{ index: 1 }, txs[3].amount, 10]
		]);
		expect(summary[0]).toBe(`Send 100 USDC to ${STRANGER_EVM}`);
		// a token transfer's estimate plus a fifth; the coin's last move costs 21,000
		expect(txs.map((t) => t.gas)).toEqual([72000, 72000, 72000, 21000]);
		// every wei is either paid as a fee at most, or moved
		const fees = txs.reduce((s, t) => s + BigInt(t.gas) * MAX_FEE, 0n);
		expect(BigInt(txs[3].amount) + fees).toBe(balance);
	});

	it('EVM: sending all of the coin keeps the fees of the moves after it', async () => {
		evmNode();
		const balance = 10n ** 18n;
		const holdings = [coin(balance), token(USDC, 5_000000n)];
		const { request } = await evm.prepare(
			intent({ from: EVM_FROM, asset: holdings[0], to: STRANGER_EVM, moveRest: 2, holdings })
		);
		const txs = request.txs as Tx[];
		expect(txs.map((t) => t.to)).toEqual([{ address: STRANGER_EVM }, { index: 2 }, { index: 2 }]);
		// kept back: a token move at 100,000 gas and the coin's move at 21,000; what the token's
		// move did not use moves last
		expect(BigInt(txs[0].amount)).toBe(balance - (21000n + 100_000n + 21000n) * MAX_FEE);
		expect(BigInt(txs[2].amount)).toBe((100_000n - 72000n) * MAX_FEE);
	});

	it('Bitcoin: sending everything pays one output and the fee', async () => {
		esploraNode([100_000]);
		const { request, summary } = await bitcoin.prepare(
			intent({ from: BTC_FROM, asset: sats(100_000n), to: STRANGER_BTC })
		);
		// one P2WPKH input, one P2WPKH output: 110 vB at 10 sat/vB
		expect(request.outputs).toEqual([{ to: { address: STRANGER_BTC }, amount: '98900' }]);
		expect(request.inputs).toEqual([
			{ txid: 'ab'.repeat(32), vout: 0, amount: '100000', prevTx: '02000000' + '00'.repeat(60) }
		]);
		expect(summary[0]).toBe(`Send 0.000989 BTC to ${STRANGER_BTC}`);
	});

	it('Bitcoin: the change goes to a fresh account; too little to keep goes to the fee', async () => {
		esploraNode([60_000, 40_000]);
		const change = await bitcoin.prepare(
			intent({
				from: BTC_FROM,
				asset: sats(100_000n),
				to: STRANGER_BTC,
				amount: 50_000n,
				moveRest: 1
			})
		);
		// two inputs, two outputs: 209 vB at 10 sat/vB
		expect(bitcoin.vsize(2, [31, 31])).toBe(209);
		expect(change.request.outputs).toEqual([
			{ to: { address: STRANGER_BTC }, amount: '50000' },
			{ to: { index: 1 }, amount: String(100_000 - 50_000 - 2090) }
		]);
		// change under the dust limit is not kept
		const dust = await bitcoin.prepare(
			intent({
				from: BTC_FROM,
				asset: sats(100_000n),
				to: STRANGER_BTC,
				amount: 97_700n,
				moveRest: 1
			})
		);
		expect(dust.request.outputs).toEqual([{ to: { address: STRANGER_BTC }, amount: '97700' }]);
		expect(dust.fee).toMatch(/^0\.000023 BTC/);
	});

	it('Bitcoin: refuses an amount under the dust limit, and more coins than one request spends', async () => {
		esploraNode([100_000]);
		await expect(
			bitcoin.prepare(
				intent({
					from: BTC_FROM,
					asset: sats(100_000n),
					to: STRANGER_BTC,
					amount: 293n,
					moveRest: 1
				})
			)
		).rejects.toThrow(/dust limit of 294 sat/);
		esploraNode(Array(201).fill(1000));
		await expect(
			bitcoin.prepare(intent({ from: BTC_FROM, asset: sats(201_000n), to: STRANGER_BTC }))
		).rejects.toThrow(/201 coins/);
	});
});

describe('moving everything', () => {
	afterEach(() => vi.unstubAllGlobals());

	it('EVM: every token, then the coin less every fee, to the fresh account; no one paid', async () => {
		evmNode();
		const balance = 10n ** 18n;
		const holdings = [coin(balance), token(USDC, 1000_000000n), token(DAI, 0n)];
		const { request, summary } = await evm.prepare(
			intent({ from: EVM_FROM, asset: holdings[0], to: '', moveRest: 3, moveAll: true, holdings })
		);
		const txs = request.txs as Tx[];
		expect(txs.map((t) => [t.to, t.amount, t.nonce])).toEqual([
			[{ index: 3 }, '1000000000', 7],
			[{ index: 3 }, txs[1].amount, 8]
		]);
		expect(summary[0]).toBe('Move 1,000 USDC to your account #3');
		const fees = txs.reduce((s, t) => s + BigInt(t.gas) * MAX_FEE, 0n);
		expect(BigInt(txs[1].amount) + fees).toBe(balance);
	});

	it('EVM: refuses when the coin cannot pay a fee, or there is no account to go to', async () => {
		evmNode();
		await expect(
			evm.prepare(
				intent({ from: EVM_FROM, asset: coin(1000n), to: '', moveRest: 1, moveAll: true })
			)
		).rejects.toThrow(/Nothing to move/);
		await expect(
			evm.prepare(intent({ from: EVM_FROM, asset: coin(10n ** 18n), to: '', moveAll: true }))
		).rejects.toThrow(/an account to go to/);
	});

	it('Bitcoin: every coin, less the fee, to the fresh account', async () => {
		esploraNode([60_000, 40_000]);
		const { request, summary } = await bitcoin.prepare(
			intent({ from: BTC_FROM, asset: sats(100_000n), to: '', moveRest: 2, moveAll: true })
		);
		// two inputs, one P2WPKH output: 178 vB at 10 sat/vB
		expect(request.outputs).toEqual([
			{ to: { index: 2 }, amount: String(100_000 - 10 * bitcoin.vsize(2, [31])) }
		]);
		expect(summary[0]).toMatch(/^Move 0\.000\d+ BTC to your account #2$/);
	});
});

describe('values', () => {
	const base = registry.evm.find((c) => c.chainId === 8453)!;
	const sepolia = registry.evm.find((c) => c.testnet)!;
	const on = (chain: typeof eth, t?: (typeof eth.tokens)[number], amount = 1n): Asset => ({
		key: `${chain.chainId}:${t?.address ?? 'native'}`,
		chain,
		...(t ? { token: t } : {}),
		symbol: t?.symbol ?? chain.symbol,
		decimals: t?.decimals ?? chain.decimals,
		amount
	});

	it('price only what Airkeys lists', () => {
		expect(priceKey(on(eth))).toBe('ETH');
		expect(priceKey(on(base))).toBe('ETH');
		expect(priceKey(sats(1n))).toBe('BTC');
		expect(priceKey(on(eth, USDC))).toBe('USDC');
		expect(
			priceKey(
				on(
					eth,
					eth.tokens.find((t) => t.symbol === 'WBTC')
				)
			)
		).toBe('BTC');
		// a token at another address that calls itself USDC, a test network, a chain added by hand
		expect(priceKey(on(eth, { ...USDC, address: STRANGER_EVM }))).toBeNull();
		expect(priceKey(on(sepolia))).toBeNull();
		expect(priceKey(on({ ...eth, chainId: 59144, name: 'Linea' }))).toBeNull();
	});

	it('read a feed answer, refusing stale or non-positive prices', () => {
		const w = (n: bigint) => BigInt.asUintN(256, n).toString(16).padStart(64, '0');
		const round = (answer: bigint, updated: number) =>
			'0x' + w(1n) + w(answer) + w(0n) + w(BigInt(updated)) + w(1n);
		const now = 1_800_000_000;
		expect(readRound(round(232_468_418_576n, now - 60), now)).toBeCloseTo(2324.68418576);
		expect(readRound(round(1n, now - STALE_SECONDS - 1), now)).toBeNull();
		expect(readRound(round(-5n, now), now)).toBeNull();
		expect(readRound(round(0n, now), now)).toBeNull();
		expect(readRound('0x', now)).toBeNull();
		expect(readRound(null, now)).toBeNull();
	});

	it('sum what sits behind hidden and exposed keys, and list what should move', () => {
		const a = (index: number) => ({ network: 'evm' as const, index, address: `0x${index}` });
		const holders: Holder[] = [
			{
				account: a(0),
				exposure: 'exposed',
				partial: false,
				assets: [
					on(eth, undefined, 10n ** 18n),
					on(
						base,
						base.tokens.find((t) => t.symbol === 'USDC'),
						5_000000n
					)
				]
			},
			{
				account: a(1),
				exposure: 'hidden',
				partial: false,
				assets: [on(eth, USDC, 100_000000n), on(eth, undefined, 0n)]
			},
			{
				account: a(2),
				exposure: 'unknown',
				partial: true,
				assets: [on(eth, { ...USDC, address: STRANGER_EVM, symbol: 'FAKE' }, 9n)]
			}
		];
		const price: Record<string, number> = { ETH: 2000, USDC: 1 };
		const valueOf = (x: Asset) => {
			const k = priceKey(x);
			return k && price[k] !== undefined ? (Number(x.amount) / 10 ** x.decimals) * price[k] : null;
		};
		const s = summarize(holders, valueOf);
		expect([s.total, s.bunker, s.exposed, s.unknown]).toEqual([2105, 100, 2005, 0]);
		expect([s.bunkerAccounts, s.exposedAccounts]).toEqual([1, 1]);
		expect(s.unpriced).toEqual(['FAKE']);
		// by worth; one line per asset and chain, across accounts; the unpriced last
		expect(s.lines.map((l) => [l.asset.symbol, l.asset.chain.name, l.value])).toEqual([
			['ETH', 'Ethereum', 2000],
			['USDC', 'Ethereum', 100],
			['USDC', 'Base', 5],
			['FAKE', 'Ethereum', null]
		]);
		// the exposed account: one entry per chain it holds something on
		expect(s.toMove.map((m) => [m.account.index, m.chain.name, m.value])).toEqual([
			[0, 'Ethereum', 2000],
			[0, 'Base', 5]
		]);
		expect(elsewhere(holders[0].assets, eth).map((e) => e.chain.name)).toEqual(['Base']);
	});
});
