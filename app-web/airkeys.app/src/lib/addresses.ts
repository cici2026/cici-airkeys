// Recipient addresses, checked as strictly as the phone checks them (the phone has the last
// word: anything it would refuse is refused here first, with the same reason).
import { keccak_256 } from '@noble/hashes/sha3.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { ripemd160 } from '@noble/hashes/legacy.js';
import { base58check, bech32, bech32m, hex } from '@scure/base';
import type { Network } from './registry';

const b58c = base58check(sha256);
const utf8 = new TextEncoder();

// ------------------------------------------------------------------ EVM

export function evmChecksum(addr: string): string {
	const lower = addr.toLowerCase().replace(/^0x/, '');
	const h = keccak_256(utf8.encode(lower));
	let out = '0x';
	for (let i = 0; i < 40; i++) {
		const nibble = (h[i >> 1] >> (i % 2 === 0 ? 4 : 0)) & 15;
		out += nibble >= 8 ? lower[i].toUpperCase() : lower[i];
	}
	return out;
}

/** The checksummed form, or an error sentence. */
export function parseEvm(s: string): string {
	if (!/^0x[0-9a-fA-F]{40}$/.test(s))
		throw new Error(`${s} is not an EVM address (0x and 40 hex digits).`);
	const body = s.slice(2);
	const mixed = /[a-f]/.test(body) && /[A-F]/.test(body);
	const sum = evmChecksum(s);
	if (mixed && sum !== s) throw new Error(`${s} has a wrong checksum: a character is mistyped.`);
	return sum;
}

// ------------------------------------------------------------------ Bitcoin

export type BtcKind = 'p2pkh' | 'p2sh' | 'p2wpkh' | 'p2wsh' | 'p2tr';

/** Output bytes of each kind (value, script length, script): for size estimates. */
export const BTC_OUTPUT_BYTES: Record<BtcKind, number> = {
	p2pkh: 34,
	p2sh: 32,
	p2wpkh: 31,
	p2wsh: 43,
	p2tr: 43
};
export const BTC_DUST: Record<BtcKind, number> = {
	p2pkh: 546,
	p2sh: 540,
	p2wpkh: 294,
	p2wsh: 330,
	p2tr: 330
};

export function parseBitcoin(s: string): { kind: BtcKind; script: Uint8Array } {
	if (/^bc1/i.test(s)) {
		const lower = s.toLowerCase();
		if (s !== lower && s !== s.toUpperCase())
			throw new Error(`${s} is not a valid Bitcoin address.`);
		const v0 = /^bc1q/.test(lower);
		try {
			const { prefix, words } = (v0 ? bech32 : bech32m).decode(lower as `${string}1${string}`, 90);
			if (prefix !== 'bc') throw new Error();
			const version = words[0];
			const program = (v0 ? bech32 : bech32m).fromWords(words.slice(1));
			if (version === 0 && program.length === 20)
				return { kind: 'p2wpkh', script: Uint8Array.of(0, 20, ...program) };
			if (version === 0 && program.length === 32)
				return { kind: 'p2wsh', script: Uint8Array.of(0, 32, ...program) };
			if (version === 1 && program.length === 32)
				return { kind: 'p2tr', script: Uint8Array.of(0x51, 32, ...program) };
		} catch {
			/* fall through */
		}
		throw new Error(`${s} is not a Bitcoin mainnet address Airkeys pays to.`);
	}
	try {
		const d = b58c.decode(s);
		if (d.length === 21 && d[0] === 0x00)
			return { kind: 'p2pkh', script: Uint8Array.of(0x76, 0xa9, 20, ...d.slice(1), 0x88, 0xac) };
		if (d.length === 21 && d[0] === 0x05)
			return { kind: 'p2sh', script: Uint8Array.of(0xa9, 20, ...d.slice(1), 0x87) };
	} catch {
		/* fall through */
	}
	throw new Error(`${s} is not a valid Bitcoin mainnet address.`);
}

/** Whether `to` pays the key behind the P2WPKH address `own`: as itself in any case, or as the
 * legacy (P2PKH) or nested (P2SH-P2WPKH) address of the same key (PROTOCOL §7.4). */
export function paysSameKey(own: string, to: string): boolean {
	const key = parseBitcoin(own).script.slice(2);
	const nested = ripemd160(sha256(Uint8Array.of(0, 20, ...key)));
	const scripts = [
		Uint8Array.of(0, 20, ...key),
		Uint8Array.of(0x76, 0xa9, 20, ...key, 0x88, 0xac),
		Uint8Array.of(0xa9, 20, ...nested, 0x87)
	].map((s) => hex.encode(s));
	return scripts.includes(hex.encode(parseBitcoin(to).script));
}

// ------------------------------------------------------------------ any network

/** The address in its canonical spelling, or an error sentence. */
export function parseAddress(network: Network, s: string): string {
	const t = s.trim();
	switch (network) {
		case 'evm':
			return parseEvm(t);
		case 'bitcoin':
			parseBitcoin(t);
			return /^bc1/i.test(t) ? t.toLowerCase() : t;
	}
}
