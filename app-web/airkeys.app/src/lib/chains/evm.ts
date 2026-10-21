// EVM chains over JSON-RPC: balances, whether the key is exposed, transfer requests (§5.2),
// broadcasting.
import { parseEvm } from '../addresses';
import { signRequest } from '../protocol';
import { formatUnits } from '../units';
import type { Chain, Token } from '../registry';
import {
	own,
	rpc,
	rpcBatch,
	type Asset,
	type Broadcast,
	type Prepared,
	type SendIntent
} from './net';

const pad = (hex: string) => hex.replace(/^0x/, '').toLowerCase().padStart(64, '0');
const big = (hex: string | null) => (hex && hex !== '0x' ? BigInt(hex) : 0n);

/** Calls per JSON-RPC batch: public endpoints refuse much larger ones. */
const BATCH = 50;

/** The balances (native coin and listed tokens) and the nonce of each address on one chain,
 * in batches; a nonce above zero means the address has signed. */
export async function scan(
	chain: Chain,
	addresses: string[]
): Promise<Map<string, { assets: Asset[]; nonce: bigint | null }>> {
	const per = 2 + chain.tokens.length;
	const group = Math.max(1, Math.floor(BATCH / per));
	const out = new Map<string, { assets: Asset[]; nonce: bigint | null }>();
	for (let i = 0; i < addresses.length; i += group) {
		// a pause between batches to one endpoint keeps under public rate limits
		if (i) await new Promise((ok) => setTimeout(ok, 300));
		const some = addresses.slice(i, i + group);
		const res = await rpcBatch(
			chain.rpc,
			some.flatMap((a): [string, unknown[]][] => [
				['eth_getBalance', [a, 'latest']],
				['eth_getTransactionCount', [a, 'latest']],
				...chain.tokens.map((t): [string, unknown[]] => [
					'eth_call',
					[{ to: t.address, data: '0x70a08231' + pad(a) }, 'latest']
				])
			])
		);
		some.forEach((a, k) => {
			const r = res.slice(k * per, (k + 1) * per);
			out.set(a, {
				assets: [
					{
						key: `${chain.chainId}:native`,
						chain,
						symbol: chain.symbol,
						decimals: chain.decimals,
						amount: big(r[0])
					},
					...chain.tokens.map((t, j): Asset => ({
						key: `${chain.chainId}:${t.address}`,
						chain,
						token: t,
						symbol: t.symbol,
						decimals: t.decimals,
						amount: big(r[j + 2])
					}))
				],
				nonce: r[1] === null ? null : big(r[1])
			});
		});
	}
	return out;
}

/** The nonce of each address on one chain (above zero: it has signed there). */
export async function nonces(
	chain: Chain,
	addresses: string[]
): Promise<Map<string, bigint | null>> {
	const out = new Map<string, bigint | null>();
	for (let i = 0; i < addresses.length; i += BATCH) {
		if (i) await new Promise((ok) => setTimeout(ok, 300));
		const some = addresses.slice(i, i + BATCH);
		const res = await rpcBatch(
			chain.rpc,
			some.map((a): [string, unknown[]] => ['eth_getTransactionCount', [a, 'latest']])
		);
		some.forEach((a, k) => out.set(a, res[k] === null ? null : big(res[k])));
	}
	return out;
}

export async function balances(chain: Chain, address: string): Promise<Asset[]> {
	return (await scan(chain, [address])).get(address)!.assets;
}

type Fee = { maxFeePerGas: string; maxPriorityFeePerGas: string } | { gasPrice: string };

async function pricing(chain: Chain, speed: 0 | 1 | 2): Promise<{ fee: Fee; price: bigint }> {
	const block = await rpc<{ baseFeePerGas?: string }>(chain.rpc, 'eth_getBlockByNumber', [
		'latest',
		false
	]);
	if (block.baseFeePerGas) {
		const base = BigInt(block.baseFeePerGas);
		let tip = 1_000_000_000n;
		try {
			tip = BigInt(await rpc<string>(chain.rpc, 'eth_maxPriorityFeePerGas'));
		} catch {
			/* the default */
		}
		tip = (tip * [8n, 10n, 15n][speed]) / 10n;
		const max = base * 2n + tip;
		return {
			fee: { maxFeePerGas: max.toString(), maxPriorityFeePerGas: tip.toString() },
			price: max
		};
	}
	const price =
		(BigInt(await rpc<string>(chain.rpc, 'eth_gasPrice')) * [10n, 11n, 13n][speed]) / 10n;
	return { fee: { gasPrice: price.toString() }, price };
}

const transferData = (to: string, amount: bigint) =>
	'0xa9059cbb' + pad(to) + amount.toString(16).padStart(64, '0');

/** A gas limit with room: the estimate plus a fifth, or plain 21000 for a coin to a wallet. */
/** OP-stack chains (Base, OP Mainnet) charge every transaction a layer-1 data fee besides its
 * gas, taken from the same balance. Their fee oracle: */
const L1_ORACLE = '0x420000000000000000000000000000000000000F';

/** The most the layer-1 data fee of one transfer can cost (0 on chains without one), with half
 * again as margin: the layer-1 price moves before the transaction is included. */
async function l1Fee(chain: Chain): Promise<bigint> {
	const code = await rpc<string>(chain.rpc, 'eth_getCode', [L1_ORACLE, 'latest']);
	if (!code || code === '0x') return 0n;
	// getL1FeeUpperBound(uint256 unsignedTxSize): a token transfer is about 110 bytes
	const bound = await rpc<string>(chain.rpc, 'eth_call', [
		{ to: L1_ORACLE, data: '0xf1c7a58b' + (200).toString(16).padStart(64, '0') },
		'latest'
	]);
	return (BigInt(bound) * 3n) / 2n;
}

async function gasFor(
	chain: Chain,
	from: string,
	to: string,
	value: bigint,
	data?: string
): Promise<number> {
	const est = Number(
		BigInt(
			await rpc<string>(chain.rpc, 'eth_estimateGas', [
				{ from, to, value: '0x' + value.toString(16), ...(data ? { data } : {}) }
			])
		)
	);
	return est === 21000 ? 21000 : Math.ceil(est * 1.2);
}

const tokenRef = (t: Token) => ({ address: t.address, symbol: t.symbol, decimals: t.decimals });

export async function prepare(i: SendIntent): Promise<Prepared> {
	const { chain } = i.asset;
	const from = i.from.address;
	const to = parseEvm(i.to);
	// a stand-in recipient for estimates of transfers to an account named by number: a random
	// address, so a new holder — the estimate covers the costlier first deposit (a well-known
	// address may already hold the token, and would under-estimate by ~17,000 gas)
	const probe =
		'0x' +
		Array.from(crypto.getRandomValues(new Uint8Array(20)), (b) =>
			b.toString(16).padStart(2, '0')
		).join('');
	const [{ fee, price }, nonceHex, l1, coinGas] = await Promise.all([
		pricing(chain, i.speed),
		rpc<string>(chain.rpc, 'eth_getTransactionCount', [from, 'pending']),
		l1Fee(chain),
		// a plain transfer of the native coin: 21,000 gas on most chains, more on Arbitrum
		gasFor(chain, from, probe, 1n)
	]);
	let nonce = Number(BigInt(nonceHex));
	const txs: Record<string, unknown>[] = [];
	const summary: string[] = [];
	let fees = 0n; // the most all transactions can cost in fees
	let values = 0n; // native coin sent
	const balance = i.holdings.find((a) => !a.token)?.amount ?? 0n;
	const room = () => balance - fees - values;
	/** The most one transaction can cost: its gas, and its layer-1 data fee where there is one. */
	const cost = (gas: number | bigint) => BigInt(gas) * price + l1;

	const push = (tx: Record<string, unknown>, gas: number, line: string) => {
		txs.push({ ...tx, nonce: nonce++, gas, ...fee });
		fees += cost(gas);
		if (!tx.token) values += BigInt(tx.amount as string);
		summary.push(line);
	};

	// the transfer asked for first; tokens that move next; the native coin that moves last
	const moving =
		i.moveRest === null
			? []
			: i.holdings.filter((a) => a.token && a.amount > 0n && a.key !== i.asset.key);
	if (i.asset.token) {
		const amount = i.amount ?? i.asset.amount;
		if (amount <= 0n || amount > i.asset.amount)
			throw new Error(
				`The account holds ${formatUnits(i.asset.amount, i.asset.decimals)} ${i.asset.symbol}.`
			);
		const gas = await gasFor(chain, from, i.asset.token.address, 0n, transferData(to, amount));
		push(
			{ to: { address: to }, amount: amount.toString(), token: tokenRef(i.asset.token) },
			gas,
			`Send ${formatUnits(amount, i.asset.decimals)} ${i.asset.symbol} to ${to}`
		);
		if (i.moveRest !== null && amount < i.asset.amount)
			moving.unshift({ ...i.asset, amount: i.asset.amount - amount });
	} else {
		const gas = await gasFor(chain, from, to, 0n);
		// sending everything, or the rest moving after: keep the fees of what follows
		const later =
			BigInt(moving.length) * cost(100_000) + (i.moveRest !== null ? cost(coinGas) : 0n);
		const amount = i.amount ?? balance - cost(gas) - later;
		if (amount <= 0n) throw new Error('The balance does not cover the network fee.');
		if (amount + cost(gas) > balance)
			throw new Error(
				`The account holds ${formatUnits(balance, chain.decimals)} ${chain.symbol}: too little for this amount and its fee.`
			);
		push(
			{ to: { address: to }, amount: amount.toString() },
			gas,
			`Send ${formatUnits(amount, chain.decimals)} ${chain.symbol} to ${to}`
		);
	}
	for (const a of moving) {
		const gas = await gasFor(chain, from, a.token!.address, 0n, transferData(probe, a.amount));
		push(
			{ to: own(i.moveRest!), amount: a.amount.toString(), token: tokenRef(a.token!) },
			gas,
			`Move ${formatUnits(a.amount, a.decimals)} ${a.symbol} to your account #${i.moveRest}`
		);
	}
	if (i.moveRest !== null) {
		const left = room() - cost(coinGas);
		if (left > 0n)
			push(
				{ to: own(i.moveRest), amount: left.toString() },
				coinGas,
				`Move ${formatUnits(left, chain.decimals)} ${chain.symbol} to your account #${i.moveRest}`
			);
	}
	if (room() < 0n)
		throw new Error(
			`The fees need up to ${formatUnits(fees, chain.decimals)} ${chain.symbol}; the account holds ${formatUnits(balance, chain.decimals)}.`
		);
	const request = signRequest('evm', i.from, { chainId: chain.chainId, txs });
	return { request, summary, fee: `≤ ${formatUnits(fees, chain.decimals)} ${chain.symbol}` };
}

export async function broadcast(chain: Chain, raws: string[]): Promise<Broadcast[]> {
	const out: Broadcast[] = [];
	for (const raw of raws) {
		try {
			out.push({ hash: await rpc<string>(chain.rpc, 'eth_sendRawTransaction', [raw]), ok: true });
		} catch (e) {
			out.push({ hash: '', ok: false, error: (e as Error).message });
		}
	}
	return out;
}
