// The web app's own code against the phone's core: the same addresses accepted and refused,
// the same transaction sizes (vectors.json comes from
// core/wallet/examples/vectors.rs; regenerate it when the core changes).
import { describe, expect, it } from 'vitest';
import { hex } from '@scure/base';
import vectors from './vectors.json';
import { parseBitcoin, parseEvm } from './addresses';
import { vsize } from './chains/bitcoin';
import { formatShort, formatUnits, parseUnits } from './units';
import { parseMessage, encode } from './protocol';

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
		expect(vsize(3, [43, 34, 32, 31])).toBe(355);
	});
});

describe('messages from the phone', () => {
	const accounts = {
		airkeys: 1,
		type: 'accounts',
		accounts: [{ network: 'evm', index: 0, address: '0xcf6335E1C68B1F7C9D8Aa84c647FFcd9595f0b03' }]
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
						{ network: 'bitcoin', index: 1, address: 'bc1q8d2x9494zdyx6ka8pp4p94xe8jpsef77pzl7td' }
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
