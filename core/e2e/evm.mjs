// EVM end to end: an anvil fork of Ethereum mainnet. Account #0 gets ETH, USDC and USDT (the
// tokens' real contracts, balances written into their storage); the phone signs a batch —
// pay USDC to a stranger, move USDT and ETH to account #1 — and the fork executes it.
// Then a legacy (gasPrice) transaction, and requests the phone must refuse.
//
//   node e2e/evm.mjs            (needs anvil and cast from Foundry, and network access)
import { execFileSync } from 'node:child_process';
import { address, buildSigner, check, done, phone, phoneRefuses, receipt as waitReceipt, rpc, start } from './lib.mjs';

const FORK = process.env.EVM_FORK_URL ?? 'https://ethereum-rpc.publicnode.com';
const PORT = 8547;
const URL = `http://127.0.0.1:${PORT}`;
const USDC = '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48';
const USDT = '0xdAC17F958D2ee523a2206206994597C13D831ec7';
const STRANGER = '0x000000000000000000000000000000000000dEaD';

const cast = (...a) => execFileSync('cast', a).toString().trim();
const hex = (n) => '0x' + BigInt(n).toString(16);

buildSigner();
const anvil = await start('anvil', ['--fork-url', FORK, '--port', String(PORT), '--chain-id', '1'], 'Listening on');
try {
	const a0 = address('evm', 0);
	const a1 = address('evm', 1);
	console.log(`account #0 ${a0}\naccount #1 ${a1}`);

	await rpc(URL, 'anvil_setBalance', [a0, hex(10n * 10n ** 18n)]);
	// balances live in a mapping: USDC (FiatToken) at slot 9, USDT (TetherToken) at slot 2
	for (const [token, slot, amount] of [[USDC, 9, 1_000_000_000n], [USDT, 2, 500_000_000n]]) {
		const key = cast('index', 'address', a0, String(slot));
		await rpc(URL, 'anvil_setStorageAt', [token, key, '0x' + amount.toString(16).padStart(64, '0')]);
	}
	const balanceOf = async (token, who) => BigInt(await rpc(URL, 'eth_call', [{ to: token, data: '0x70a08231' + who.slice(2).toLowerCase().padStart(64, '0') }, 'latest']));
	check((await balanceOf(USDC, a0)) === 1_000_000_000n, 'account #0 holds 1000 USDC on the fork');

	const nonce = Number(await rpc(URL, 'eth_getTransactionCount', [a0, 'pending']));
	const block = await rpc(URL, 'eth_getBlockByNumber', ['latest', false]);
	const maxFee = (BigInt(block.baseFeePerGas) * 2n + 10n ** 9n).toString();
	const fee = { maxFeePerGas: maxFee, maxPriorityFeePerGas: '1000000000' };
	const request = {
		airkeys: 1, type: 'sign', id: 'evm-batch', network: 'evm', chainId: 1,
		from: { index: 0, address: a0 },
		txs: [
			{ to: { address: STRANGER }, amount: '100000000', token: { address: USDC, symbol: 'USDC', decimals: 6 }, nonce, gas: 100000, ...fee },
			{ to: { index: 1 }, amount: '50000000', token: { address: USDT, symbol: 'USDT', decimals: 6 }, nonce: nonce + 1, gas: 100000, ...fee },
			{ to: { index: 1, address: a1 }, amount: '1500000000000000000', nonce: nonce + 2, gas: 21000, ...fee },
		],
	};
	const usdcBefore = await balanceOf(USDC, STRANGER);
	const signed = phone([], request);
	check(signed.type === 'signed' && signed.id === 'evm-batch' && signed.txs.length === 3, 'the phone signs the batch');
	check(signed.accounts?.length === 1 && signed.accounts[0].index === 1 && signed.accounts[0].address === a1, '`accounts` reports account #1, named by index alone');
	for (const [i, tx] of signed.txs.entries()) {
		const hash = await rpc(URL, 'eth_sendRawTransaction', [tx.raw]);
		check(hash === tx.hash, `tx ${i + 1}: the node computes the same hash`);
		const receipt = await waitReceipt(URL, hash);
		check(receipt?.status === '0x1', `tx ${i + 1}: executed successfully (type ${receipt?.type})`);
		const onchain = await rpc(URL, 'eth_getTransactionByHash', [hash]);
		check(onchain.from.toLowerCase() === a0.toLowerCase(), `tx ${i + 1}: sent by account #0`);
	}
	check((await balanceOf(USDC, STRANGER)) - usdcBefore === 100_000_000n, 'the stranger received 100 USDC');
	check((await balanceOf(USDT, a1)) === 50_000_000n, 'account #1 received 50 USDT');
	check(BigInt(await rpc(URL, 'eth_getBalance', [a1, 'latest'])) === 1_500_000_000_000_000_000n, 'account #1 received 1.5 ETH');

	// legacy, EIP-155
	const legacy = {
		airkeys: 1, type: 'sign', id: 'evm-legacy', network: 'evm', chainId: 1,
		from: { index: 0, address: a0 },
		txs: [{ to: { address: STRANGER }, amount: '1', nonce: nonce + 3, gas: 21000, gasPrice: maxFee }],
	};
	const s2 = phone([], legacy);
	const h2 = await rpc(URL, 'eth_sendRawTransaction', [s2.txs[0].raw]);
	const r2 = await waitReceipt(URL, h2);
	check(r2?.status === '0x1' && r2.type === '0x0', 'a legacy transaction executes');

	// requests the phone refuses
	const wrongChecksum = structuredClone(request);
	wrongChecksum.txs[0].to.address = '0x000000000000000000000000000000000000dEAD';
	check(/checksum/.test(phoneRefuses(wrongChecksum)), 'refuses a recipient with a wrong checksum');
	const fakeUsdc = structuredClone(request);
	fakeUsdc.txs[0].token.decimals = 18;
	check(/but it is USDC with 6 decimals/.test(phoneRefuses(fakeUsdc)), 'refuses a listed token with the wrong decimals');
	const gap = structuredClone(request);
	gap.txs[1].nonce += 1;
	check(/consecutive/.test(phoneRefuses(gap)), 'refuses nonces with a gap');
	const back = structuredClone(request);
	back.txs[2].to = { index: 0 };
	check(/goes back to the account that pays/.test(phoneRefuses(back)), 'refuses a transfer back to the paying account');
	const other = structuredClone(request);
	other.from.address = a1;
	check(/for another wallet/.test(phoneRefuses(other)), 'refuses a request for another wallet');
	const lying = structuredClone(request);
	lying.txs[2].to.address = STRANGER;
	check(/this wallet's account #1 is/.test(phoneRefuses(lying)), 'refuses an own-account reference with a foreign address');
} finally {
	anvil.kill();
}
done();
